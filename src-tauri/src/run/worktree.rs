//! Worktrees git de uma run: criação, instalação de dependências, commit e remoção.
//!
//! Hooks do repositório ficam desligados em todos os comandos do orquestrador
//! (`core.hooksPath=/dev/null`): um hook alterado pelo agente não pode rodar no host.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::process::{self, Options};

const GIT_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(300);
const OUTPUT_LIMIT: usize = 8 * 1024;
/// Listas de arquivos (`ls-files`, `grep -l`) podem ser longas em repositórios grandes.
const LIST_LIMIT: usize = 512 * 1024;
const MAX_SLUG: usize = 40;
const HOOKS_OFF: &str = "core.hooksPath=/dev/null";

fn git(git: &Path, dirs: &[PathBuf], cwd: &Path, args: &[&str]) -> Result<String, String> {
    let mut full = vec!["-c", HOOKS_OFF];
    full.extend_from_slice(args);
    let options = Options {
        cwd: Some(cwd),
        timeout: GIT_TIMEOUT,
        limit: OUTPUT_LIMIT,
    };
    let output = process::run_with(git, &full, dirs, &options)?;
    if output.succeeded() {
        Ok(output.text.trim().to_owned())
    } else {
        let reason: String = output.text.trim().chars().take(300).collect();
        Err(format!(
            "git {}: {reason}",
            args.first().copied().unwrap_or("")
        ))
    }
}

pub fn head_sha(git_bin: &Path, dirs: &[PathBuf], repo: &Path) -> Result<String, String> {
    git(git_bin, dirs, repo, &["rev-parse", "HEAD"])
}

/// SHA completo de uma revisão (`HEAD`, branch ou SHA); `None` se o git não a conhece.
pub fn resolve_rev(git_bin: &Path, dirs: &[PathBuf], repo: &Path, rev: &str) -> Option<String> {
    git(
        git_bin,
        dirs,
        repo,
        &["rev-parse", "--verify", "--quiet", rev],
    )
    .ok()
}

/// SHA do último commit de uma branch local (`refs/heads/<branch>`).
pub fn branch_sha(
    git_bin: &Path,
    dirs: &[PathBuf],
    repo: &Path,
    branch: &str,
) -> Result<String, String> {
    let reference = format!("refs/heads/{branch}");
    git(git_bin, dirs, repo, &["rev-parse", "--verify", &reference])
}

pub fn create(
    git_bin: &Path,
    dirs: &[PathBuf],
    repo: &Path,
    dir: &Path,
    branch: &str,
    base_sha: &str,
) -> Result<(), String> {
    let dir = dir.to_string_lossy();
    git(
        git_bin,
        dirs,
        repo,
        &["worktree", "add", "-b", branch, &dir, base_sha],
    )
    .map(|_| ())
}

/// Recria o worktree de uma branch que já existe. O worktree de uma run é removido ao
/// terminar; a branch fica, e é dela que o ambiente de teste volta a nascer.
pub fn checkout(
    git_bin: &Path,
    dirs: &[PathBuf],
    repo: &Path,
    dir: &Path,
    branch: &str,
) -> Result<(), String> {
    // Registro de um worktree cuja pasta já sumiu impediria criar outro no mesmo lugar.
    let _ = git(git_bin, dirs, repo, &["worktree", "prune"]);
    let dir = dir.to_string_lossy();
    git(git_bin, dirs, repo, &["worktree", "add", &dir, branch]).map(|_| ())
}

/// Instala dependências quando o projeto usa Bun com lockfile.
pub fn install_deps(bun: Option<&Path>, dirs: &[PathBuf], dir: &Path) -> Result<(), String> {
    let uses_bun = dir.join("package.json").is_file()
        && (dir.join("bun.lock").is_file() || dir.join("bun.lockb").is_file());
    let Some(bun) = bun.filter(|_| uses_bun) else {
        return Ok(());
    };
    let options = Options {
        cwd: Some(dir),
        timeout: INSTALL_TIMEOUT,
        limit: OUTPUT_LIMIT,
    };
    let output = process::run_with(bun, &["install", "--frozen-lockfile"], dirs, &options)?;
    if output.succeeded() {
        Ok(())
    } else {
        let reason: String = output.text.trim().chars().take(300).collect();
        Err(format!("bun install: {reason}"))
    }
}

/// Commita o que sobrou no worktree. `true` se a branch tem commits além de `base_sha`.
pub fn commit_all(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base_sha: &str,
    message: &str,
) -> Result<bool, String> {
    git(git_bin, dirs, dir, &["add", "-A"])?;
    if !git(git_bin, dirs, dir, &["status", "--porcelain"])?.is_empty() {
        git(
            git_bin,
            dirs,
            dir,
            &["commit", "--no-verify", "-m", message],
        )?;
    }
    let range = format!("{base_sha}..HEAD");
    let count = git(git_bin, dirs, dir, &["rev-list", "--count", &range])?;
    Ok(count != "0")
}

/// Arquivos rastreados do repositório (caminhos relativos).
pub fn ls_files(git_bin: &Path, dirs: &[PathBuf], repo: &Path) -> Result<Vec<String>, String> {
    let options = Options {
        cwd: Some(repo),
        timeout: GIT_TIMEOUT,
        limit: LIST_LIMIT,
    };
    let output = process::run_with(git_bin, &["-c", HOOKS_OFF, "ls-files"], dirs, &options)?;
    if !output.succeeded() {
        return Err("git ls-files falhou.".into());
    }
    Ok(output.text.lines().map(str::to_owned).collect())
}

/// Arquivos rastreados cujo conteúdo contém `term` (sem diferenciar maiúsculas).
/// Nenhum casamento não é erro: devolve lista vazia.
pub fn grep_files(
    git_bin: &Path,
    dirs: &[PathBuf],
    repo: &Path,
    term: &str,
) -> Result<Vec<String>, String> {
    let options = Options {
        cwd: Some(repo),
        timeout: GIT_TIMEOUT,
        limit: LIST_LIMIT,
    };
    let args = ["-c", HOOKS_OFF, "grep", "-l", "-i", "-F", "-I", "-e", term];
    let output = process::run_with(git_bin, &args, dirs, &options)?;
    match output.status {
        Some(0) => Ok(output.text.lines().map(str::to_owned).collect()),
        Some(1) => Ok(Vec::new()),
        _ => Err("git grep falhou.".into()),
    }
}

/// Diff unificado entre a base e `head` (o HEAD do worktree ou uma branch), com `context`
/// linhas de contexto. Saída limitada a [`LIST_LIMIT`]; quem entrega ao agente corta no
/// próprio orçamento.
pub fn diff_range(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base_sha: &str,
    head: &str,
    context: u32,
) -> Result<String, String> {
    let options = Options {
        cwd: Some(dir),
        timeout: GIT_TIMEOUT,
        limit: LIST_LIMIT,
    };
    let unified = format!("-U{context}");
    let args = [
        "-c",
        HOOKS_OFF,
        "diff",
        "--no-color",
        "--no-ext-diff",
        &unified,
        base_sha,
        head,
    ];
    let output = process::run_with(git_bin, &args, dirs, &options)?;
    if output.succeeded() {
        Ok(output.text)
    } else {
        Err("git diff falhou.".into())
    }
}

/// Resumo das mudanças da run em relação à base (`git diff --stat`).
pub fn diff_stat(
    git_bin: &Path,
    dirs: &[PathBuf],
    dir: &Path,
    base_sha: &str,
) -> Result<String, String> {
    git(git_bin, dirs, dir, &["diff", "--stat", base_sha])
}

pub fn remove(git_bin: &Path, dirs: &[PathBuf], repo: &Path, dir: &Path) -> Result<(), String> {
    let dir = dir.to_string_lossy();
    git(
        git_bin,
        dirs,
        repo,
        &["worktree", "remove", "--force", &dir],
    )
    .map(|_| ())
}

/// Trecho do título seguro para nome de branch: `[a-z0-9-]`, sem repetição de hífen.
pub fn slug(title: &str) -> String {
    let mut slug = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= MAX_SLUG {
            break;
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "tarefa".into()
    } else {
        slug.to_owned()
    }
}

pub fn branch_name(run_id: i64, title: &str) -> String {
    format!("harnessy/run-{run_id}-{}", slug(title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_branch_safe() {
        assert_eq!(slug("Ajustar o texto, já!"), "ajustar-o-texto-j");
        assert_eq!(slug("¿¿¿"), "tarefa");
        assert!(slug(&"a".repeat(200)).len() <= MAX_SLUG);
        assert_eq!(branch_name(3, "Fix: bug"), "harnessy/run-3-fix-bug");
    }

    #[test]
    fn creates_worktree_and_commits_changes() {
        let dirs = process::search_dirs();
        let Some(git_bin) = process::find_binary("git", &dirs) else {
            return;
        };
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("harnessy-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "t@example.com"],
            vec!["config", "user.name", "Teste"],
        ] {
            git(&git_bin, &dirs, &repo, &args).unwrap();
        }
        std::fs::write(repo.join("a.txt"), "a").unwrap();
        git(&git_bin, &dirs, &repo, &["add", "-A"]).unwrap();
        git(&git_bin, &dirs, &repo, &["commit", "-q", "-m", "base"]).unwrap();
        let base = head_sha(&git_bin, &dirs, &repo).unwrap();
        assert_eq!(
            resolve_rev(&git_bin, &dirs, &repo, "HEAD"),
            Some(base.clone())
        );
        assert_eq!(resolve_rev(&git_bin, &dirs, &repo, "nao-existe"), None);

        let wt = root.join("wt");
        create(&git_bin, &dirs, &repo, &wt, "harnessy/run-1-x", &base).unwrap();
        assert!(!commit_all(&git_bin, &dirs, &wt, &base, "nada").unwrap());
        std::fs::write(wt.join("b.txt"), "b").unwrap();
        assert!(commit_all(&git_bin, &dirs, &wt, &base, "mudança").unwrap());
        remove(&git_bin, &dirs, &repo, &wt).unwrap();
        assert!(!wt.exists());

        // O worktree some ao fim da run, mas a branch fica: dá para recriá-lo no mesmo lugar.
        checkout(&git_bin, &dirs, &repo, &wt, "harnessy/run-1-x").unwrap();
        assert!(wt.join("b.txt").is_file(), "a mudança da run volta junto");
        // Pasta apagada à mão (sem `git worktree remove`) também não impede recriar.
        std::fs::remove_dir_all(&wt).unwrap();
        checkout(&git_bin, &dirs, &repo, &wt, "harnessy/run-1-x").unwrap();
        assert!(wt.join("b.txt").is_file());
        assert!(checkout(
            &git_bin,
            &dirs,
            &repo,
            &root.join("outro"),
            "branch-que-nao-existe"
        )
        .is_err());
        remove(&git_bin, &dirs, &repo, &wt).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
