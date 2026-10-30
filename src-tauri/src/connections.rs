//! Detecção das conexões GitHub, Codex e Claude Code.
//!
//! O Harnessy reaproveita a autenticação já feita pelo usuário nas CLIs
//! oficiais. Nenhuma credencial é lida, copiada ou persistida: o núcleo apenas
//! executa o comando de status de cada ferramenta e resume o resultado.

use serde::Serialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(20);
const OUTPUT_LIMIT: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Github,
    Codex,
    Claude,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// CLI instalada e sessão válida.
    Connected,
    /// CLI instalada, mas sem sessão válida (nunca autenticou ou expirou).
    Expired,
    /// CLI não encontrada neste computador.
    Unavailable,
    /// A verificação falhou ou demorou demais; o estado real é desconhecido.
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub provider: Provider,
    pub state: State,
    /// Frase curta para a interface, sem dados sensíveis.
    pub detail: String,
    /// Conta identificada pela CLI, quando disponível.
    pub account: Option<String>,
    /// Caminho da CLI encontrada, para diagnóstico.
    pub binary: Option<String>,
    /// Comando que o usuário executa no terminal para autenticar.
    pub login_command: &'static str,
    /// Comando para instalar a CLI, quando ausente.
    pub install_hint: &'static str,
}

struct Probe {
    provider: Provider,
    binary: &'static str,
    args: &'static [&'static str],
    login_command: &'static str,
    install_hint: &'static str,
}

const PROBES: [Probe; 3] = [
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

/// Diretórios onde CLIs costumam ficar. Aplicativos GUI no macOS não herdam o
/// PATH do shell, então o PATH do processo é apenas um dos candidatos.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for relative in [".local/bin", ".bun/bin", ".npm-global/bin", ".cargo/bin"] {
            dirs.push(home.join(relative));
        }
    }
    for fixed in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        dirs.push(PathBuf::from(fixed));
    }
    dirs
}

fn find_binary(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

struct Output {
    status: Option<i32>,
    text: String,
}

/// Executa a CLI com tempo limite. Entrada fechada: a CLI não pode abrir um
/// fluxo interativo de login a partir daqui.
fn run(binary: &Path, args: &[&str], dirs: &[PathBuf]) -> Result<Output, String> {
    let path = std::env::join_paths(dirs).map_err(|error| error.to_string())?;
    let mut child = Command::new(binary)
        .args(args)
        .env("PATH", path)
        .env("NO_COLOR", "1")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("não foi possível executar a CLI: {error}"))?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        for stream in [
            stdout.as_mut().map(|s| s as &mut dyn Read),
            stderr.as_mut().map(|s| s as &mut dyn Read),
        ]
        .into_iter()
        .flatten()
        {
            let _ = stream.take(OUTPUT_LIMIT as u64).read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if started.elapsed() < TIMEOUT => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("a verificação demorou demais e foi interrompida".into());
            }
            Err(error) => return Err(format!("falha ao aguardar a CLI: {error}")),
        }
    };
    let text = reader.join().unwrap_or_default();
    Ok(Output { status, text })
}

fn account_after(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let account: String = text[start..]
        .trim_start()
        .trim_start_matches('"')
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '(' && *c != '"' && *c != ',')
        .collect();
    (!account.is_empty()).then_some(account)
}

/// Interpreta a saída de cada CLI. Função pura para permitir testes sem
/// executar processos.
fn interpret(provider: Provider, output: &Output) -> (State, String, Option<String>) {
    let ok = output.status == Some(0);
    let text = output.text.as_str();
    match provider {
        Provider::Github => {
            let account = account_after(text, "Logged in to github.com account");
            if ok && text.contains("Logged in to") {
                (
                    State::Connected,
                    "Sessão do GitHub CLI válida.".into(),
                    account,
                )
            } else if text.contains("not logged") || text.contains("You are not logged") {
                (
                    State::Expired,
                    "GitHub CLI instalado, mas sem sessão ativa.".into(),
                    None,
                )
            } else if ok {
                (
                    State::Connected,
                    "GitHub CLI respondeu com sessão ativa.".into(),
                    account,
                )
            } else {
                (
                    State::Expired,
                    "O GitHub CLI não confirmou a sessão. Autentique novamente.".into(),
                    None,
                )
            }
        }
        Provider::Codex => {
            let lower = text.to_ascii_lowercase();
            if ok && lower.contains("logged in") && !lower.contains("not logged in") {
                let method = if lower.contains("chatgpt") {
                    "conta ChatGPT"
                } else {
                    "chave de API"
                };
                (
                    State::Connected,
                    format!("Codex CLI autenticado com {method}."),
                    None,
                )
            } else if lower.contains("not logged in") || !ok {
                (
                    State::Expired,
                    "Codex CLI instalado, mas sem sessão ativa.".into(),
                    None,
                )
            } else {
                (
                    State::Unknown,
                    "Resposta inesperada do Codex CLI.".into(),
                    None,
                )
            }
        }
        Provider::Claude => {
            let logged_in =
                text.contains("\"loggedIn\": true") || text.contains("\"loggedIn\":true");
            if ok && logged_in {
                let method = account_after(text, "\"authMethod\":")
                    .map(|m| m.trim_matches('"').to_string())
                    .unwrap_or_else(|| "sessão local".into());
                (
                    State::Connected,
                    format!("Claude Code autenticado ({method})."),
                    None,
                )
            } else if ok || text.contains("loggedIn") {
                (
                    State::Expired,
                    "Claude Code instalado, mas sem sessão ativa.".into(),
                    None,
                )
            } else {
                (
                    State::Unknown,
                    "Resposta inesperada do Claude Code.".into(),
                    None,
                )
            }
        }
    }
}

fn check(probe: &Probe, dirs: &[PathBuf]) -> ConnectionStatus {
    let base = |state, detail: String, account, binary: Option<&Path>| ConnectionStatus {
        provider: probe.provider,
        state,
        detail,
        account,
        binary: binary.map(|path| path.to_string_lossy().into_owned()),
        login_command: probe.login_command,
        install_hint: probe.install_hint,
    };
    let Some(binary) = find_binary(probe.binary, dirs) else {
        return base(
            State::Unavailable,
            "CLI não encontrada neste computador.".into(),
            None,
            None,
        );
    };
    match run(&binary, probe.args, dirs) {
        Ok(output) => {
            let (state, detail, account) = interpret(probe.provider, &output);
            base(state, detail, account, Some(&binary))
        }
        Err(reason) => base(
            State::Unknown,
            format!("Não foi possível verificar: {reason}."),
            None,
            Some(&binary),
        ),
    }
}

/// Verifica as três conexões em paralelo. Comando assíncrono para não bloquear
/// a thread principal enquanto as CLIs respondem.
#[tauri::command]
pub async fn check_connections() -> Vec<ConnectionStatus> {
    let dirs = search_dirs();
    let handles: Vec<_> = PROBES
        .iter()
        .map(|probe| {
            let dirs = dirs.clone();
            std::thread::spawn(move || check(probe, &dirs))
        })
        .collect();
    handles
        .into_iter()
        .zip(PROBES.iter())
        .map(|(handle, probe)| {
            handle.join().unwrap_or_else(|_| ConnectionStatus {
                provider: probe.provider,
                state: State::Unknown,
                detail: "A verificação falhou inesperadamente.".into(),
                account: None,
                binary: None,
                login_command: probe.login_command,
                install_hint: probe.install_hint,
            })
        })
        .collect()
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
        let (state, _, account) = interpret(Provider::Github, &ok);
        assert_eq!(state, State::Connected);
        assert_eq!(account.as_deref(), Some("octocat"));
        let missing = out(
            1,
            "You are not logged into any GitHub hosts. Run gh auth login.",
        );
        assert_eq!(interpret(Provider::Github, &missing).0, State::Expired);
    }

    #[test]
    fn codex_status_is_interpreted() {
        assert_eq!(
            interpret(Provider::Codex, &out(0, "Logged in using ChatGPT")).0,
            State::Connected
        );
        assert_eq!(
            interpret(Provider::Codex, &out(1, "Not logged in")).0,
            State::Expired
        );
    }

    #[test]
    fn claude_status_is_interpreted() {
        let ok = out(
            0,
            "{\n  \"loggedIn\": true,\n  \"authMethod\": \"claude.ai\"\n}",
        );
        let (state, detail, _) = interpret(Provider::Claude, &ok);
        assert_eq!(state, State::Connected);
        assert!(detail.contains("claude.ai"));
        assert_eq!(
            interpret(Provider::Claude, &out(0, "{\"loggedIn\": false}")).0,
            State::Expired
        );
    }

    #[test]
    fn missing_binary_is_unavailable() {
        let status = check(&PROBES[1], &[PathBuf::from("/nonexistent-harnessy")]);
        assert_eq!(status.state, State::Unavailable);
        assert_eq!(status.login_command, "codex login");
    }
}
