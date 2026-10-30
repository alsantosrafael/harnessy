//! Verificações do projeto, executadas pelo núcleo e nunca pelo agente.
//!
//! O resultado dos checks é a evidência que decide se uma run termina ou recebe
//! mais uma tentativa de correção. Este módulo é puro, exceto a execução dos
//! checks, que passa pelo mesmo mecanismo de processos das demais verificações.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::failure::{self, cut_at, Failure};
use crate::process;

/// Tentativas de correção depois da primeira implementação (SPEC §8).
pub const MAX_CORRECTIONS: u32 = 3;
/// Bytes guardados de cada check para o parser de falhas: o final da saída.
const PARSE_LIMIT: usize = 64 * 1024;
/// Bytes lidos de cada check. O processo guarda só o início da saída, então
/// saídas maiores que isto perdem o final antes do parser.
const OUTPUT_LIMIT: usize = 256 * 1024;
/// Orçamento total do prompt de correção.
const PROMPT_LIMIT: usize = 8 * 1024;
/// Falhas com seção própria no prompt; as demais aparecem só na contagem.
const DETAIL_LIMIT: usize = 5;
/// Tamanho máximo de cada valor (teste, esperado, obtido) no prompt.
const VALUE_LIMIT: usize = 300;
/// Bytes reservados por seção para o rótulo "Trecho:" no cálculo do orçamento.
const EXCERPT_LABEL_RESERVE: usize = 16;
const FIX_INSTRUCTION: &str = "Corrija o código para que estes checks passem. Não remova nem enfraqueça testes. Termine com um resumo curto do que corrigiu.\n";

/// Papel de um check para a contraprova (SPEC §5.2): os de teste provam a mudança;
/// os de tipo servem de defesa contra API inventada e também são rodados nela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    Types,
    Tests,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub kind: CheckKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub name: String,
    pub passed: bool,
    pub output: String,
    /// Comando do check: o parser de falhas escolhe o formato da saída por ele.
    program: String,
    args: Vec<String>,
}

/// Checks do perfil de verificação, detectados na raiz do repositório (SPEC §4.2).
/// Tipos antes de Testes; todos são obrigatórios.
pub fn checks_for(repo: &Path) -> Vec<Check> {
    let read = |name: &str| std::fs::read_to_string(repo.join(name)).ok();
    checks_from(
        read("package.json").as_deref(),
        repo.join("Cargo.toml").is_file(),
        read("pyproject.toml").as_deref(),
    )
}

/// Regras de detecção, sobre o conteúdo dos manifests (testável sem disco):
/// - script `typecheck` (ou, na falta, `check`) do `package.json` → Tipos;
/// - script `test` do `package.json` → Testes;
/// - `Cargo.toml` → Tipos `cargo check`, e Testes `cargo test` só sem Testes do `package.json`;
/// - `pyproject.toml` com `[tool.mypy]` ou `[tool.pyright]` → Tipos via `uv run`.
fn checks_from(package: Option<&str>, cargo: bool, pyproject: Option<&str>) -> Vec<Check> {
    let scripts = package
        .and_then(|manifest| serde_json::from_str::<Value>(manifest).ok())
        .map(|value| value["scripts"].clone())
        .unwrap_or(Value::Null);
    let has = |script: &str| scripts[script].is_string();
    let mut types = Vec::new();
    let mut tests = Vec::new();
    if has("typecheck") {
        types.push(bun_run("typecheck"));
    } else if has("check") {
        types.push(bun_run("check"));
    }
    if has("test") {
        tests.push(bun_run("test"));
    }
    if cargo {
        types.push(cargo_check());
        if tests.is_empty() {
            tests.push(cargo_test());
        }
    }
    if let Some(toml) = pyproject.and_then(|text| toml::from_str::<toml::Table>(text).ok()) {
        let tool = toml.get("tool").and_then(toml::Value::as_table);
        let typed = |tool_name: &str| tool.is_some_and(|tool| tool.contains_key(tool_name));
        let typer = if typed("mypy") {
            Some("mypy")
        } else if typed("pyright") {
            Some("pyright")
        } else {
            None
        };
        if let Some(typer) = typer {
            types.push(Check {
                name: "Tipos".into(),
                program: "uv".into(),
                args: vec!["run".into(), typer.into(), ".".into()],
                kind: CheckKind::Types,
            });
        }
    }
    types.extend(tests);
    types
}

fn bun_run(script: &str) -> Check {
    let (name, kind) = if script == "test" {
        ("Testes", CheckKind::Tests)
    } else {
        ("Tipos", CheckKind::Types)
    };
    Check {
        name: name.into(),
        program: "bun".into(),
        args: vec!["run".into(), script.into()],
        kind,
    }
}

fn cargo_check() -> Check {
    Check {
        name: "Tipos".into(),
        program: "cargo".into(),
        args: vec!["check".into()],
        kind: CheckKind::Types,
    }
}

fn cargo_test() -> Check {
    Check {
        name: "Testes".into(),
        program: "cargo".into(),
        args: vec!["test".into()],
        kind: CheckKind::Tests,
    }
}

/// Executa todos os checks no diretório da run. Falha só se um binário não existir.
pub fn run_all(checks: &[Check], dirs: &[PathBuf], cwd: &Path) -> Result<Vec<Report>, String> {
    checks
        .iter()
        .map(|check| {
            let binary = process::find_binary(&check.program, dirs)
                .ok_or_else(|| format!("{} não encontrado.", check.program))?;
            let args: Vec<&str> = check.args.iter().map(String::as_str).collect();
            let options = process::Options {
                cwd: Some(cwd),
                limit: OUTPUT_LIMIT,
                ..process::Options::default()
            };
            let output = process::run_with(&binary, &args, dirs, &options)?;
            Ok(Report {
                name: check.name.clone(),
                passed: output.succeeded(),
                output: tail(&output.text, PARSE_LIMIT),
                program: check.program.clone(),
                args: check.args.clone(),
            })
        })
        .collect()
}

pub fn failing(reports: &[Report]) -> Vec<&Report> {
    reports.iter().filter(|report| !report.passed).collect()
}

/// Falhas estruturadas de todos os checks que falharam, na ordem dos checks.
pub(super) fn failures(reports: &[&Report]) -> Vec<Failure> {
    reports
        .iter()
        .flat_map(|report| {
            failure::parse(&report.name, &report.program, &report.args, &report.output)
        })
        .collect()
}

/// Prompt de correção: a tarefa, uma seção por falha (arquivo:linha, teste,
/// esperado, obtido, trecho) e a instrução de não enfraquecer testes. Cabe em
/// [`PROMPT_LIMIT`]; com mais de [`DETAIL_LIMIT`] falhas, o resto vira contagem.
pub fn correction_prompt(title: &str, failing: &[&Report]) -> String {
    let head = format!(
        "A verificação do projeto falhou depois da sua alteração na tarefa \"{title}\".\n\n"
    );
    failure_prompt(head, &failures(failing))
}

/// Prompt de correção a partir de falhas já estruturadas (checks ou revisão).
pub(super) fn failure_prompt(head: String, failures: &[Failure]) -> String {
    let total = failures.len();
    let detailed = &failures[..total.min(DETAIL_LIMIT)];
    let more = total.saturating_sub(DETAIL_LIMIT);
    let closing = if more > 0 {
        format!("… e mais {more} falhas\n\n{FIX_INSTRUCTION}")
    } else {
        FIX_INSTRUCTION.to_owned()
    };
    // Medida sem trecho; o que sobra do orçamento é dividido entre os trechos.
    let fixed: usize = detailed
        .iter()
        .enumerate()
        .map(|(at, failure)| section(at, total, failure, 0).len() + EXCERPT_LABEL_RESERVE)
        .sum();
    let room = PROMPT_LIMIT.saturating_sub(head.len() + closing.len() + fixed);
    let per_excerpt = room.checked_div(detailed.len()).unwrap_or(0);
    let mut prompt = head;
    for (at, failure) in detailed.iter().enumerate() {
        prompt.push_str(&section(at, total, failure, per_excerpt));
    }
    prompt.push_str(&closing);
    cut_at(&prompt, PROMPT_LIMIT).to_owned()
}

/// Seção de uma falha no formato do prompt de correção.
fn section(at: usize, total: usize, failure: &Failure, excerpt_limit: usize) -> String {
    let mut text = format!("## Falha {} de {} — {}\n", at + 1, total, failure.check);
    if let Some(location) = location(failure) {
        text.push_str(&format!("Arquivo: {location}\n"));
    }
    if let Some(name) = &failure.test_name {
        text.push_str(&format!("Teste: {}\n", quoted(name)));
    }
    if let Some(value) = &failure.expected {
        text.push_str(&format!("Esperado: {}\n", quoted(value)));
    }
    if let Some(value) = &failure.actual {
        text.push_str(&format!("Obtido: {}\n", quoted(value)));
    }
    let excerpt = cut_at(failure.excerpt.trim(), excerpt_limit);
    if !excerpt.is_empty() {
        text.push_str(&format!("Trecho:\n{excerpt}\n"));
    }
    text.push('\n');
    text
}

/// `arquivo:linha`, ou só o arquivo quando não há linha.
fn location(failure: &Failure) -> Option<String> {
    match (&failure.file, failure.line) {
        (Some(file), Some(line)) => Some(format!("{file}:{line}")),
        (Some(file), None) => Some(file.clone()),
        (None, _) => None,
    }
}

/// Valor entre aspas, com quebras de linha escritas como `\n` e tamanho limitado.
fn quoted(value: &str) -> String {
    format!("\"{}\"", cut_at(value, VALUE_LIMIT).replace('\n', "\\n"))
}

/// Mantém o final do texto (onde ficam os resumos de falha), respeitando UTF-8.
fn tail(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("[…]\n{}", &text[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(name: &str, passed: bool, output: &str) -> Report {
        Report {
            name: name.into(),
            passed,
            output: output.into(),
            program: "fake".into(),
            args: Vec::new(),
        }
    }

    /// Relatório de `tsc` com `errors` erros de tipo, cada linha com 1500 bytes.
    fn tsc_report(errors: usize) -> Report {
        let output: String = (1..=errors)
            .map(|i| format!("src/a{i}.ts({i},1): error TS2322: {}\n", "x".repeat(1500)))
            .collect();
        Report {
            name: "Tipos".into(),
            passed: false,
            output,
            program: "tsc".into(),
            args: Vec::new(),
        }
    }

    /// Forma compacta de um check para comparar: `kind program args...`.
    fn shape(check: &Check) -> String {
        format!(
            "{:?} {} {}",
            check.kind,
            check.program,
            check.args.join(" ")
        )
    }

    fn shapes(package: Option<&str>, cargo: bool, pyproject: Option<&str>) -> Vec<String> {
        checks_from(package, cargo, pyproject)
            .iter()
            .map(shape)
            .collect()
    }

    #[test]
    fn bun_test_script_becomes_a_tests_check() {
        assert_eq!(
            shapes(Some(r#"{"scripts":{"test":"bun test"}}"#), false, None),
            ["Tests bun run test"]
        );
        assert!(shapes(Some(r#"{"scripts":{"build":"x"}}"#), false, None).is_empty());
        assert!(shapes(Some("não é json"), false, None).is_empty());
        assert!(shapes(None, false, None).is_empty());
    }

    #[test]
    fn bun_typecheck_comes_before_tests_and_prefers_typecheck_over_check() {
        let both = r#"{"scripts":{"test":"bun test","typecheck":"tsc","check":"x"}}"#;
        assert_eq!(
            shapes(Some(both), false, None),
            ["Types bun run typecheck", "Tests bun run test"]
        );
        let only_check = r#"{"scripts":{"check":"tsc --noEmit"}}"#;
        assert_eq!(
            shapes(Some(only_check), false, None),
            ["Types bun run check"]
        );
    }

    #[test]
    fn cargo_gives_check_and_test_unless_package_json_has_tests() {
        assert_eq!(
            shapes(None, true, None),
            ["Types cargo check", "Tests cargo test"]
        );
        let with_tests = r#"{"scripts":{"test":"bun test"}}"#;
        assert_eq!(
            shapes(Some(with_tests), true, None),
            ["Types cargo check", "Tests bun run test"]
        );
    }

    #[test]
    fn uv_project_with_mypy_or_pyright_gets_a_types_check() {
        let mypy = "[tool.mypy]\nstrict = true\n";
        assert_eq!(shapes(None, false, Some(mypy)), ["Types uv run mypy ."]);
        let pyright = "[tool.pyright]\n";
        assert_eq!(
            shapes(None, false, Some(pyright)),
            ["Types uv run pyright ."]
        );
        assert!(shapes(None, false, Some("[project]\nname = \"x\"\n")).is_empty());
        assert!(shapes(None, false, Some("não é toml =")).is_empty());
    }

    #[test]
    fn failing_keeps_only_failed_reports() {
        let reports = [report("a", true, ""), report("b", false, "erro")];
        let failing = failing(&reports);
        assert_eq!(failing.len(), 1);
        assert_eq!(failing[0].name, "b");
    }

    #[test]
    fn correction_prompt_names_only_failing_checks() {
        let ok = report("Lint", true, "lint ok");
        let bad = report("Testes", false, "expect(1).toBe(2)");
        let prompt = correction_prompt("Somar", &[&bad]);
        assert!(prompt.contains("\"Somar\""));
        assert!(prompt.contains("## Falha 1 de 1 — Testes"));
        assert!(prompt.contains("expect(1).toBe(2)"));
        assert!(!prompt.contains(&ok.output));
    }

    #[test]
    fn prompt_with_seven_failures_shows_five_cites_lines_and_fits() {
        let report = tsc_report(7);
        let prompt = correction_prompt("Somar", &[&report]);
        assert!(prompt.len() <= PROMPT_LIMIT);
        assert!(prompt.contains("## Falha 5 de 7 — Tipos"));
        assert!(!prompt.contains("## Falha 6"));
        assert!(prompt.contains("… e mais 2 falhas"));
        assert!(prompt.contains("Arquivo: src/a1.ts:1"));
        assert!(prompt.contains("Não remova nem enfraqueça testes"));
    }

    #[test]
    fn failures_come_from_each_failing_report_with_its_check_name() {
        let report = tsc_report(2);
        let found = failures(&[&report]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].check, "Tipos");
        assert_eq!(found[1].line, Some(2));
    }

    #[test]
    fn run_all_reports_the_exit_status_of_each_check() {
        let dirs = process::search_dirs();
        let checks = [
            Check {
                name: "ok".into(),
                program: "true".into(),
                args: Vec::new(),
                kind: CheckKind::Tests,
            },
            Check {
                name: "falha".into(),
                program: "false".into(),
                args: Vec::new(),
                kind: CheckKind::Tests,
            },
        ];
        let reports = run_all(&checks, &dirs, Path::new("/")).unwrap();
        assert_eq!(reports.len(), 2);
        assert!(reports[0].passed);
        assert!(!reports[1].passed);
        assert_eq!(failing(&reports)[0].name, "falha");
    }

    #[test]
    fn tail_keeps_the_end_on_a_char_boundary() {
        let text = "é".repeat(10);
        let cut = tail(&text, 5);
        assert!(cut.starts_with("[…]\n"));
        assert_eq!(cut, "[…]\néé");
        assert_eq!(tail("curto", 100), "curto");
    }
}
