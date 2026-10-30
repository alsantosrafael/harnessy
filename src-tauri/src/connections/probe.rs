//! Tabela estática das CLIs verificadas. Apenas comandos de status, nunca login.

use super::Provider;

pub struct Probe {
    pub provider: Provider,
    pub binary: &'static str,
    pub args: &'static [&'static str],
    pub login_command: &'static str,
    pub install_hint: &'static str,
}

pub const PROBES: [Probe; 3] = [
    Probe {
        provider: Provider::Github,
        binary: "gh",
        args: &["auth", "status", "--hostname", "github.com"],
        login_command: "gh auth login --web",
        install_hint: "brew install gh",
    },
    Probe {
        provider: Provider::Codex,
        binary: "codex",
        args: &["login", "status"],
        login_command: "codex login",
        install_hint: "bun install -g @openai/codex",
    },
    Probe {
        provider: Provider::Claude,
        binary: "claude",
        args: &["auth", "status"],
        login_command: "claude auth login",
        install_hint: "curl -fsSL https://claude.ai/install.sh | bash",
    },
];
