//! Efeitos no GitHub de uma run concluída: `git push`, abrir, ler e fazer merge do PR.
//!
//! Tudo roda em Rust com o login que o usuário já fez em `git` e `gh`; o agente
//! nunca recebe essas ferramentas (ADR 0001). Os argumentos vêm de dados que o
//! Harnessy controla (nome de branch gerado, número do PR), nunca da WebView.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use super::redact;
use crate::{
    error::{AppError, Result},
    process::{self, Options},
};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(90);
const OUTPUT_LIMIT: usize = 64 * 1024;
const MAX_REASON_CHARS: usize = 300;
const HOOKS_OFF: &str = "core.hooksPath=/dev/null";

/// Binários e pasta onde os comandos rodam.
pub struct Tools<'a> {
    pub git: &'a Path,
    pub gh: &'a Path,
    pub dirs: &'a [PathBuf],
    pub repo: &'a Path,
}

impl Tools<'_> {
    fn exec(&self, binary: &Path, args: &[&str]) -> Result<String> {
        let options = Options {
            cwd: Some(self.repo),
            timeout: NETWORK_TIMEOUT,
            limit: OUTPUT_LIMIT,
        };
        let output = process::run_with(binary, args, self.dirs, &options).map_err(failure)?;
        if output.succeeded() {
            return Ok(output.text.trim().to_owned());
        }
        Err(failure(output.text))
    }

    /// Envia a branch para `origin`, sem force e sem hooks do repositório.
    pub fn push(&self, branch: &str) -> Result<()> {
        self.exec(
            self.git,
            &["-c", HOOKS_OFF, "push", "--set-upstream", "origin", branch],
        )
        .map(|_| ())
    }

    /// PR já existente para a branch (aberto, fechado ou mergeado), em JSON de `gh`.
    pub fn find_pr(&self, branch: &str) -> Result<Option<String>> {
        let text = self.exec(
            self.gh,
            &[
                "pr",
                "list",
                "--head",
                branch,
                "--state",
                "all",
                "--limit",
                "1",
                "--json",
                crate::delivery::PR_FIELDS,
            ],
        )?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&text)
            .map_err(|_| AppError::Delivery("Resposta inesperada do GitHub.".into()))?;
        Ok(items.into_iter().next().map(|item| item.to_string()))
    }

    /// Abre o PR (draft por padrão) e devolve a URL.
    pub fn create_pr(&self, branch: &str, title: &str, body: &str, draft: bool) -> Result<String> {
        let mut args = vec![
            "pr", "create", "--head", branch, "--title", title, "--body", body,
        ];
        if draft {
            args.push("--draft");
        }
        self.exec(self.gh, &args)
    }

    pub fn view_pr(&self, number: i64) -> Result<String> {
        self.exec(
            self.gh,
            &[
                "pr",
                "view",
                &number.to_string(),
                "--json",
                crate::delivery::PR_FIELDS,
            ],
        )
    }

    /// Tira o PR de draft.
    pub fn mark_ready(&self, number: i64) -> Result<()> {
        self.exec(self.gh, &["pr", "ready", &number.to_string()])
            .map(|_| ())
    }

    /// Squash merge. A branch local fica (o worktree da run ainda a usa).
    pub fn merge(&self, number: i64, expected_sha: &str) -> Result<()> {
        self.exec(
            self.gh,
            &[
                "pr",
                "merge",
                &number.to_string(),
                "--squash",
                "--match-head-commit",
                expected_sha,
            ],
        )
        .map(|_| ())
    }
}

/// Frase curta e sem segredos a partir do texto de erro da CLI.
fn failure(reason: impl ToString) -> AppError {
    let clean = redact::clean(&reason.to_string());
    let line: String = clean.trim().chars().take(MAX_REASON_CHARS).collect();
    AppError::Delivery(if line.is_empty() {
        "O GitHub recusou a operação.".to_owned()
    } else {
        line
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_is_short_and_never_empty() {
        assert!(failure("").to_string().contains("recusou"));
        let long = "x".repeat(2_000);
        assert!(failure(long).to_string().chars().count() <= MAX_REASON_CHARS);
    }
}
