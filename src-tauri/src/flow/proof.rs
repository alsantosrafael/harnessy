//! Contraprova do teste (SPEC §5.2): prova se algum teste novo ou alterado falha
//! sem a mudança de código. Roda no worktree da run, depois do commit, sem tokens.
//!
//! Fluxo: lista os arquivos alterados entre a base e o HEAD da run e separa testes
//! de código; restaura só o código para a base; roda os checks de teste e de tipo;
//! volta o worktree ao HEAD da run. O resultado é aviso: não muda o estado da run.
//!
//! Limite conhecido: testes inline (`#[cfg(test)]` dentro do arquivo de código) são
//! classificados como código, então alterá-los não conta como teste alterado.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::json;

use super::{
    failure::Failure,
    verify::{self, Check, CheckKind, Report},
};
use crate::process::{self, Options};

const GIT_TIMEOUT: Duration = Duration::from_secs(60);
/// Lista de arquivos alterados: limite alto para não perder entradas de um commit grande.
const LIST_LIMIT: usize = 1024 * 1024;
const GIT_LIMIT: usize = 8 * 1024;
const HOOKS_OFF: &str = "core.hooksPath=/dev/null";

/// Veredito gravado em `runs.proof` e mostrado na interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Um teste novo ou alterado falha sem a mudança.
    Proved,
    /// Os checks de teste passam sem a mudança: o teste não prova nada.
    NotProved,
    /// A run não alterou nenhum arquivo de teste.
    NoTests,
    /// Não rodou: Ação com `contraprova = false` ou perfil sem check de teste.
    Skipped,
    /// Rodou, mas não deu para concluir (falha de infraestrutura ou falhas alheias aos testes).
    Inconclusive,
}

impl Verdict {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::NotProved => "not_proved",
            Self::NoTests => "no_tests",
            Self::Skipped => "skipped",
            Self::Inconclusive => "inconclusive",
        }
    }
}

/// Um arquivo alterado entre a base e a run. `status` é a letra do `git diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Change {
    pub(super) status: char,
    pub(super) path: String,
}

/// Resultado da contraprova, pronto para os eventos e o ledger.
pub(super) struct Proof {
    pub(super) verdict: Verdict,
    /// Testes novos ou alterados, usados para casar as falhas.
    pub(super) test_files: Vec<String>,
    pub(super) failures: Vec<Failure>,
    /// Frase do aviso para `Skipped` e `Inconclusive`; vazia nos demais.
    note: String,
    /// Falha de infraestrutura: o worktree fica no disco para inspeção.
    pub(super) keep_worktree: bool,
}

impl Proof {
    fn new(
        verdict: Verdict,
        test_files: Vec<String>,
        failures: Vec<Failure>,
        note: &str,
        keep_worktree: bool,
    ) -> Self {
        Self {
            verdict,
            test_files,
            failures,
            note: note.to_owned(),
            keep_worktree,
        }
    }

    /// Frase do status da run para a interface.
    pub(super) fn message(&self) -> String {
        match self.verdict {
            Verdict::Proved => "Comprovado por testes que falham sem a mudança".into(),
            Verdict::NotProved => {
                "Atenção: nenhum teste prova a mudança (os testes novos passam sem ela)".into()
            }
            Verdict::NoTests => "Atenção: nenhum teste foi alterado".into(),
            Verdict::Skipped | Verdict::Inconclusive => self.note.clone(),
        }
    }

    /// Conteúdo do evento `proof`: veredito, testes considerados e falhas.
    pub(super) fn event_json(&self) -> String {
        json!({
            "verdict": self.verdict.as_str(),
            "testFiles": self.test_files,
            "failures": self.failures,
        })
        .to_string()
    }
}

/// Executa a contraprova no worktree da run. `base` é o SHA da base e o worktree
/// já está no commit da run. Nunca falha: falhas de infraestrutura viram veredito.
pub(super) fn run(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base: &str,
    checks: &[Check],
    enabled: bool,
) -> Proof {
    if !enabled {
        return Proof::new(
            Verdict::Skipped,
            Vec::new(),
            Vec::new(),
            "Contraprova pulada: a Ação declara contraprova = false.",
            false,
        );
    }
    if !checks.iter().any(|check| check.kind == CheckKind::Tests) {
        return Proof::new(
            Verdict::Skipped,
            Vec::new(),
            Vec::new(),
            "Contraprova pulada: o perfil não tem check de testes.",
            false,
        );
    }
    prove(git_bin, dirs, dir, base, checks).unwrap_or_else(|reason| {
        Proof::new(
            Verdict::Inconclusive,
            Vec::new(),
            Vec::new(),
            &format!("Contraprova não concluída: {reason}"),
            true,
        )
    })
}

fn prove(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base: &str,
    checks: &[Check],
) -> Result<Proof, String> {
    let listed = git(
        git_bin,
        dirs,
        dir,
        &["diff", "--name-status", "-z", "--no-renames", base, "HEAD"],
        LIST_LIMIT,
    )?;
    let changes = parse_changes(&listed);
    let tests = test_files(&changes);
    if tests.is_empty() {
        return Ok(Proof::new(
            Verdict::NoTests,
            Vec::new(),
            Vec::new(),
            "",
            false,
        ));
    }
    let head = git(git_bin, dirs, dir, &["rev-parse", "HEAD"], GIT_LIMIT)?
        .trim()
        .to_owned();
    let reports = restore_and_run(git_bin, dirs, dir, base, &head, &changes, checks)?;
    let failing = verify::failing(&reports);
    let failures = verify::failures(&failing);
    let verdict = judge(&tests, &failures, !failing.is_empty());
    let note = if verdict == Verdict::Inconclusive {
        "Contraprova inconclusiva: as falhas não apontam para os testes alterados."
    } else {
        ""
    };
    Ok(Proof::new(verdict, tests, failures, note, false))
}

/// Restaura o código para a base, roda os checks e volta ao HEAD. O retorno ao
/// HEAD roda mesmo se algo falhou antes, para o worktree terminar no SHA da run.
fn restore_and_run(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base: &str,
    head: &str,
    changes: &[Change],
    checks: &[Check],
) -> Result<Vec<Report>, String> {
    let restored = restore_code(git_bin, dirs, dir, base, changes);
    let reports = restored.and_then(|()| verify::run_all(checks, dirs, dir));
    let back = restore_head(git_bin, dirs, dir, head);
    match (reports, back) {
        (Ok(reports), Ok(())) => Ok(reports),
        (Err(reason), _) | (Ok(_), Err(reason)) => Err(reason),
    }
}

/// Código adicionado pela run sai do disco; código modificado ou apagado volta à base.
/// Testes ficam como estão no HEAD.
fn restore_code(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base: &str,
    changes: &[Change],
) -> Result<(), String> {
    let mut from_base: Vec<&str> = Vec::new();
    for change in changes.iter().filter(|change| !is_test_path(&change.path)) {
        if change.status == 'A' {
            match std::fs::remove_file(dir.join(&change.path)) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(format!("não removeu {}: {error}", change.path)),
            }
        } else {
            from_base.push(&change.path);
        }
    }
    if from_base.is_empty() {
        return Ok(());
    }
    let mut args = vec!["checkout", base, "--"];
    args.extend(from_base);
    git(git_bin, dirs, dir, &args, GIT_LIMIT).map(|_| ())
}

/// Devolve o worktree ao HEAD da run: arquivos rastreados, não rastreados (só
/// dentro do worktree da run) e confirmação de que o SHA não mudou.
fn restore_head(git_bin: &Path, dirs: &[PathBuf], dir: &Path, head: &str) -> Result<(), String> {
    git(
        git_bin,
        dirs,
        dir,
        &["checkout", head, "--", "."],
        GIT_LIMIT,
    )?;
    git(git_bin, dirs, dir, &["clean", "-fd"], GIT_LIMIT)?;
    if !git(git_bin, dirs, dir, &["status", "--porcelain"], GIT_LIMIT)?
        .trim()
        .is_empty()
    {
        return Err("o worktree não voltou limpo ao commit da run".into());
    }
    if git(git_bin, dirs, dir, &["rev-parse", "HEAD"], GIT_LIMIT)?.trim() != head {
        return Err("o HEAD do worktree mudou durante a contraprova".into());
    }
    Ok(())
}

/// Veredito a partir dos testes alterados e das falhas dos checks. Passou tudo:
/// `NotProved`; falhou com arquivo de teste alterado: `Proved`; senão: `Inconclusive`.
fn judge(test_files: &[String], failures: &[Failure], any_failing: bool) -> Verdict {
    if !any_failing {
        return Verdict::NotProved;
    }
    let points_to_test = failures.iter().any(|failure| {
        failure
            .file
            .as_deref()
            .is_some_and(|file| test_files.iter().any(|test| same_file(file, test)))
    });
    if points_to_test {
        Verdict::Proved
    } else {
        Verdict::Inconclusive
    }
}

/// Casa o arquivo de uma falha com um arquivo alterado: iguais, ou um é sufixo do
/// outro em fronteira de diretório (o runner pode imprimir caminho absoluto).
fn same_file(reported: &str, changed: &str) -> bool {
    let reported = reported.trim_start_matches("./");
    if reported.is_empty() || changed.is_empty() {
        return false;
    }
    reported == changed
        || reported.ends_with(&format!("/{changed}"))
        || changed.ends_with(&format!("/{reported}"))
}

/// Testes alterados ou adicionados. Apagar um teste não prova a mudança.
fn test_files(changes: &[Change]) -> Vec<String> {
    changes
        .iter()
        .filter(|change| matches!(change.status, 'A' | 'M') && is_test_path(&change.path))
        .map(|change| change.path.clone())
        .collect()
}

/// Lê a saída de `git diff --name-status -z --no-renames`: pares `estado\0caminho\0`.
fn parse_changes(text: &str) -> Vec<Change> {
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        if let Some(status) = status.chars().next() {
            changes.push(Change {
                status,
                path: path.to_owned(),
            });
        }
    }
    changes
}

/// Caminho de teste pelos padrões do runner (SPEC §5.2): `*.test.*`, `*.spec.*`,
/// `*_test.go`, `test_*.py`, `*_test.py`, ou qualquer segmento `tests/`, `test/`, `__tests__/`.
fn is_test_path(path: &str) -> bool {
    let mut dirs: Vec<&str> = path.split('/').collect();
    let name = dirs.pop().unwrap_or(path);
    if dirs
        .iter()
        .any(|dir| matches!(*dir, "tests" | "test" | "__tests__"))
    {
        return true;
    }
    name.contains(".test.")
        || name.contains(".spec.")
        || name.ends_with("_test.go")
        || name.ends_with("_test.py")
        || (name.starts_with("test_") && name.ends_with(".py"))
}

/// Comando git no worktree da run, com hooks desligados (como em `run::worktree`).
fn git(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    args: &[&str],
    limit: usize,
) -> Result<String, String> {
    let mut full = vec!["-c", HOOKS_OFF];
    full.extend_from_slice(args);
    let options = Options {
        cwd: Some(dir),
        timeout: GIT_TIMEOUT,
        limit,
    };
    let output = process::run_with(git_bin, &full, dirs, &options)?;
    if output.succeeded() {
        Ok(output.text)
    } else {
        let reason: String = output.text.trim().chars().take(200).collect();
        Err(format!(
            "git {}: {reason}",
            args.first().copied().unwrap_or("")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(file: Option<&str>) -> Failure {
        Failure {
            check: "Testes".into(),
            file: file.map(str::to_owned),
            line: Some(1),
            test_name: None,
            expected: None,
            actual: None,
            excerpt: String::new(),
        }
    }

    fn change(status: char, path: &str) -> Change {
        Change {
            status,
            path: path.into(),
        }
    }

    #[test]
    fn test_paths_follow_the_runner_patterns() {
        for path in [
            "src/a.test.ts",
            "src/a.spec.tsx",
            "pkg/x_test.go",
            "tests/a.rs",
            "src/__tests__/a.ts",
            "test/a.rb",
            "app/test_x.py",
            "app/x_test.py",
        ] {
            assert!(is_test_path(path), "{path} deveria ser teste");
        }
        for path in [
            "src/test.ts",
            "src/testing.ts",
            "src/contest/a.rs",
            "tests_helper.py",
            "src/flow/mod.rs",
            "src/lib.rs",
        ] {
            assert!(!is_test_path(path), "{path} não deveria ser teste");
        }
    }

    #[test]
    fn changes_are_parsed_from_name_status_z_output() {
        let text = "M\0src/a.ts\0A\0tests/b.test.ts\0D\0src/c.rs\0";
        assert_eq!(
            parse_changes(text),
            [
                change('M', "src/a.ts"),
                change('A', "tests/b.test.ts"),
                change('D', "src/c.rs"),
            ]
        );
        assert!(parse_changes("").is_empty());
    }

    #[test]
    fn only_added_or_modified_test_files_count() {
        let changes = [
            change('M', "src/a.ts"),
            change('A', "tests/b.test.ts"),
            change('M', "src/c.test.ts"),
            change('D', "src/old.test.ts"),
        ];
        assert_eq!(test_files(&changes), ["tests/b.test.ts", "src/c.test.ts"]);
        assert!(test_files(&[change('M', "src/a.ts")]).is_empty());
    }

    #[test]
    fn failure_paths_match_changed_files_by_suffix() {
        assert!(same_file("src/a.test.ts", "src/a.test.ts"));
        assert!(same_file("/repo/src/a.test.ts", "src/a.test.ts"));
        assert!(same_file("./src/a.test.ts", "src/a.test.ts"));
        assert!(same_file("a.test.ts", "src/a.test.ts"));
        assert!(!same_file("src/b.test.ts", "src/a.test.ts"));
        assert!(!same_file("xa.test.ts", "a.test.ts"));
        assert!(!same_file("", "a.test.ts"));
    }

    #[test]
    fn judge_proves_only_when_a_changed_test_fails() {
        let tests = vec!["tests/a.test.ts".to_owned()];
        assert_eq!(judge(&tests, &[], false), Verdict::NotProved);
        assert_eq!(
            judge(&tests, &[failure(Some("tests/a.test.ts"))], true),
            Verdict::Proved
        );
        assert_eq!(
            judge(&tests, &[failure(Some("src/x.ts"))], true),
            Verdict::Inconclusive
        );
        assert_eq!(judge(&tests, &[failure(None)], true), Verdict::Inconclusive);
        assert_eq!(judge(&tests, &[], true), Verdict::Inconclusive);
    }

    #[test]
    fn verdicts_have_the_stored_names() {
        assert_eq!(Verdict::Proved.as_str(), "proved");
        assert_eq!(Verdict::NotProved.as_str(), "not_proved");
        assert_eq!(Verdict::NoTests.as_str(), "no_tests");
        assert_eq!(Verdict::Skipped.as_str(), "skipped");
        assert_eq!(Verdict::Inconclusive.as_str(), "inconclusive");
    }

    #[test]
    fn skipped_runs_say_why_and_keep_nothing() {
        let check = Check {
            name: "Testes".into(),
            program: "true".into(),
            args: Vec::new(),
            kind: CheckKind::Tests,
        };
        let off = run(
            Path::new("/nao-existe"),
            &[],
            Path::new("/"),
            "x",
            &[check],
            false,
        );
        assert_eq!(off.verdict, Verdict::Skipped);
        assert!(!off.keep_worktree);
        assert!(off.message().contains("contraprova = false"));

        let no_tests = run(
            Path::new("/nao-existe"),
            &[],
            Path::new("/"),
            "x",
            &[],
            true,
        );
        assert_eq!(no_tests.verdict, Verdict::Skipped);
        assert!(no_tests.message().contains("check de testes"));
    }
}
