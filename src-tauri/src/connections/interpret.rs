//! Interpretação da saída de cada CLI. Funções puras, testadas sem processos.

use super::{process::Output, Provider, State};

#[derive(Debug, PartialEq, Eq)]
pub struct Verdict {
    pub state: State,
    pub detail: String,
    pub account: Option<String>,
}

impl Verdict {
    pub fn new(state: State, detail: impl Into<String>) -> Self {
        Self {
            state,
            detail: detail.into(),
            account: None,
        }
    }

    fn with_account(mut self, account: Option<String>) -> Self {
        self.account = account;
        self
    }
}

pub fn interpret(provider: Provider, output: &Output) -> Verdict {
    match provider {
        Provider::Github => github(output),
        Provider::Codex => codex(output),
        Provider::Claude => claude(output),
    }
}

fn github(output: &Output) -> Verdict {
    let text = output.text.as_str();
    let account = account_after(text, "Logged in to github.com account");
    if output.succeeded() && text.contains("Logged in to") {
        Verdict::new(State::Connected, "Sessão do GitHub CLI válida.").with_account(account)
    } else if text.contains("not logged") || text.contains("You are not logged") {
        Verdict::new(
            State::Expired,
            "GitHub CLI instalado, mas sem sessão ativa.",
        )
    } else if output.succeeded() {
        Verdict::new(State::Connected, "GitHub CLI respondeu com sessão ativa.")
            .with_account(account)
    } else {
        Verdict::new(
            State::Expired,
            "O GitHub CLI não confirmou a sessão. Autentique novamente.",
        )
    }
}

fn codex(output: &Output) -> Verdict {
    let lower = output.text.to_ascii_lowercase();
    let not_logged = lower.contains("not logged in");
    if output.succeeded() && lower.contains("logged in") && !not_logged {
        let method = if lower.contains("chatgpt") {
            "conta ChatGPT"
        } else {
            "chave de API"
        };
        Verdict::new(
            State::Connected,
            format!("Codex CLI autenticado com {method}."),
        )
    } else if not_logged || !output.succeeded() {
        Verdict::new(State::Expired, "Codex CLI instalado, mas sem sessão ativa.")
    } else {
        Verdict::new(State::Unknown, "Resposta inesperada do Codex CLI.")
    }
}

fn claude(output: &Output) -> Verdict {
    let text = output.text.as_str();
    let logged_in = text.contains("\"loggedIn\": true") || text.contains("\"loggedIn\":true");
    if output.succeeded() && logged_in {
        let method = account_after(text, "\"authMethod\":")
            .map(|m| m.trim_matches('"').to_string())
            .unwrap_or_else(|| "sessão local".into());
        Verdict::new(
            State::Connected,
            format!("Claude Code autenticado ({method})."),
        )
    } else if output.succeeded() || text.contains("loggedIn") {
        Verdict::new(
            State::Expired,
            "Claude Code instalado, mas sem sessão ativa.",
        )
    } else {
        Verdict::new(State::Unknown, "Resposta inesperada do Claude Code.")
    }
}

/// Primeiro token após `marker`, sem aspas e sem delimitadores comuns.
fn account_after(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let account: String = text[start..]
        .trim_start()
        .trim_start_matches('"')
        .chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, '(' | '"' | ','))
        .collect();
    (!account.is_empty()).then_some(account)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(status: i32, text: &str) -> Output {
        Output {
            status: Some(status),
            text: text.into(),
        }
    }

    #[test]
    fn github_status_is_interpreted() {
        let ok = out(
            0,
            "github.com\n  ✓ Logged in to github.com account octocat (keyring)\n",
        );
        let verdict = interpret(Provider::Github, &ok);
        assert_eq!(verdict.state, State::Connected);
        assert_eq!(verdict.account.as_deref(), Some("octocat"));
        let missing = out(
            1,
            "You are not logged into any GitHub hosts. Run gh auth login.",
        );
        assert_eq!(interpret(Provider::Github, &missing).state, State::Expired);
    }

    #[test]
    fn codex_status_is_interpreted() {
        assert_eq!(
            interpret(Provider::Codex, &out(0, "Logged in using ChatGPT")).state,
            State::Connected
        );
        assert_eq!(
            interpret(Provider::Codex, &out(1, "Not logged in")).state,
            State::Expired
        );
    }

    #[test]
    fn claude_status_is_interpreted() {
        let ok = out(
            0,
            "{\n  \"loggedIn\": true,\n  \"authMethod\": \"claude.ai\"\n}",
        );
        let verdict = interpret(Provider::Claude, &ok);
        assert_eq!(verdict.state, State::Connected);
        assert!(verdict.detail.contains("claude.ai"));
        assert_eq!(
            interpret(Provider::Claude, &out(0, "{\"loggedIn\": false}")).state,
            State::Expired
        );
    }
}
