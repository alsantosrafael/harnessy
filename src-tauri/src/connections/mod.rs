//! Detecção das conexões GitHub, Codex e Claude Code.
//!
//! O Harnessy reaproveita a autenticação já feita pelo usuário nas CLIs
//! oficiais. Nenhuma credencial é lida, copiada ou persistida: o núcleo apenas
//! executa o comando de status de cada ferramenta e resume o resultado.
//!
//! - [`probe`] descreve cada CLI (binário, argumentos, dicas).
//! - [`crate::process`] localiza e executa binários com tempo limite.
//! - [`interpret`] traduz a saída em um [`Verdict`] sem tocar em processos.

mod interpret;
mod probe;

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::process;
use interpret::Verdict;
use probe::{Probe, PROBES};

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

impl ConnectionStatus {
    fn new(probe: &Probe, verdict: Verdict, binary: Option<&Path>) -> Self {
        Self {
            provider: probe.provider,
            state: verdict.state,
            detail: verdict.detail,
            account: verdict.account,
            binary: binary.map(|path| path.to_string_lossy().into_owned()),
            login_command: probe.login_command,
            install_hint: probe.install_hint,
        }
    }
}

fn check(probe: &Probe, dirs: &[PathBuf]) -> ConnectionStatus {
    let Some(binary) = process::find_binary(probe.binary, dirs) else {
        let verdict = Verdict::new(State::Unavailable, "CLI não encontrada neste computador.");
        return ConnectionStatus::new(probe, verdict, None);
    };
    let verdict = match process::run(&binary, probe.args, dirs) {
        Ok(output) => interpret::interpret(probe.provider, &output),
        Err(reason) => Verdict::new(
            State::Unknown,
            format!("Não foi possível verificar: {reason}."),
        ),
    };
    ConnectionStatus::new(probe, verdict, Some(&binary))
}

/// Verifica as três conexões em paralelo. Comando assíncrono para não bloquear
/// a thread principal enquanto as CLIs respondem.
#[tauri::command]
pub async fn check_connections() -> Vec<ConnectionStatus> {
    let dirs = process::search_dirs();
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
            handle.join().unwrap_or_else(|_| {
                let verdict = Verdict::new(State::Unknown, "A verificação falhou inesperadamente.");
                ConnectionStatus::new(probe, verdict, None)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_is_unavailable() {
        let status = check(&PROBES[1], &[PathBuf::from("/nonexistent-harnessy")]);
        assert_eq!(status.state, State::Unavailable);
        assert_eq!(status.login_command, "codex login");
    }
}
