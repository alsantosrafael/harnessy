//! Processo do agente: spawn, leitura do stream, limites e cancelamento.
//!
//! Único lugar que executa um processo de longa duração (regra 16). O
//! supervisor não conhece banco, Tauri nem o formato da saída: entrega linhas
//! brutas por callback e devolve como o processo terminou. Interpretar a saída
//! é trabalho do adaptador do runtime.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};

pub const RUN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const POLL: Duration = Duration::from_millis(100);

/// Tudo que o agente recebe: binário, argumentos, ambiente completo e prompt.
pub struct Plan {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub prompt: String,
}

/// Como o processo terminou, sem interpretar a saída.
#[derive(Debug, PartialEq, Eq)]
pub struct End {
    pub status: Option<i32>,
    pub cancelled: bool,
    pub timed_out: bool,
}

/// Linha lida do stdout ou do stderr do processo.
pub enum Line {
    Out(String),
    Err(String),
}

#[derive(Default)]
struct Slot {
    pgid: Option<i32>,
    cancelled: bool,
}

/// Runs ativas: grupo de processo e pedido de cancelamento, por id da run.
#[derive(Default)]
pub struct Registry {
    runs: Mutex<HashMap<i64, Slot>>,
}

impl Registry {
    fn with<T>(&self, run_id: i64, f: impl FnOnce(&mut Slot) -> T) -> Option<T> {
        let mut runs = self.runs.lock().ok()?;
        runs.get_mut(&run_id).map(f)
    }

    /// Marca a run como ativa, ainda antes de existir processo (fase de preparo).
    pub fn register(&self, run_id: i64) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.insert(run_id, Slot::default());
        }
    }

    /// Como [`Registry::register`], mas recusa (`false`) se a run já está registrada:
    /// duas revisões na mesma run disputariam o slot de cancelamento e o número do ciclo.
    pub fn try_register(&self, run_id: i64) -> bool {
        match self.runs.lock() {
            Ok(mut runs) if !runs.contains_key(&run_id) => {
                runs.insert(run_id, Slot::default());
                true
            }
            _ => false,
        }
    }

    pub fn unregister(&self, run_id: i64) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.remove(&run_id);
        }
    }

    pub fn is_cancelled(&self, run_id: i64) -> bool {
        self.with(run_id, |slot| slot.cancelled).unwrap_or(false)
    }

    /// Pede o cancelamento e encerra o grupo de processos, se já existir.
    /// `false` se a run não está ativa.
    pub fn cancel(&self, run_id: i64) -> bool {
        match self.with(run_id, |slot| {
            slot.cancelled = true;
            slot.pgid
        }) {
            Some(pgid) => {
                if let Some(pgid) = pgid {
                    kill_group(pgid);
                }
                true
            }
            None => false,
        }
    }
}

fn kill_group(pgid: i32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .status();
}

/// Executa o processo até o fim, cancelamento ou tempo limite. `Err` só se o
/// binário não puder ser iniciado.
pub fn run(
    plan: Plan,
    run_id: i64,
    registry: &Registry,
    mut on_line: impl FnMut(Line),
) -> Result<End, String> {
    let mut child = match Command::new(&plan.binary)
        .args(&plan.args)
        .env_clear()
        .envs(plan.env.iter().map(|(k, v)| (k, v)))
        .current_dir(&plan.cwd)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(format!("não foi possível iniciar o agente: {error}")),
    };
    let pgid = child.id() as i32;
    registry.with(run_id, |slot| slot.pgid = Some(pgid));

    if let Some(mut stdin) = child.stdin.take() {
        let prompt = plan.prompt;
        std::thread::spawn(move || {
            let _ = stdin.write_all(prompt.as_bytes());
        });
    }

    let (tx, rx) = mpsc::channel();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(Line::Out(line)).is_err() {
                    break;
                }
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                if tx.send(Line::Err(line)).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let started = Instant::now();
    let mut timed_out = false;
    let mut was_cancelled = false;
    loop {
        match rx.recv_timeout(POLL) {
            Ok(line) => on_line(line),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if registry.is_cancelled(run_id) && !was_cancelled {
            was_cancelled = true;
            kill_group(pgid);
        } else if started.elapsed() > RUN_TIMEOUT && !timed_out {
            timed_out = true;
            kill_group(pgid);
        }
    }
    let status = child.wait().ok().and_then(|exit| exit.code());
    Ok(End {
        status,
        cancelled: was_cancelled || registry.is_cancelled(run_id),
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process;

    #[test]
    fn a_run_can_only_be_registered_once_until_it_is_released() {
        let registry = Registry::default();
        assert!(registry.try_register(7));
        assert!(!registry.try_register(7), "segunda revisão da mesma run");
        assert!(registry.try_register(8), "outra run não é afetada");
        registry.unregister(7);
        assert!(registry.try_register(7));
    }

    fn plan(script: &str) -> Plan {
        let dirs = process::search_dirs();
        Plan {
            binary: process::find_binary("sh", &dirs).expect("sh"),
            args: vec!["-c".into(), script.into()],
            env: vec![("PATH".into(), "/usr/bin:/bin".into())],
            cwd: std::env::temp_dir(),
            prompt: String::new(),
        }
    }

    #[test]
    fn delivers_lines_and_isolates_environment() {
        let registry = Registry::default();
        registry.register(1);
        let mut out = Vec::new();
        let script = r#"echo "ok ${HARNESSY_X:-sem}"; echo "erro" >&2"#;
        std::env::set_var("HARNESSY_X", "vazou");
        let end = run(plan(script), 1, &registry, |line| match line {
            Line::Out(text) => out.push(format!("out:{text}")),
            Line::Err(text) => out.push(format!("err:{text}")),
        })
        .unwrap();
        assert_eq!(end.status, Some(0));
        assert!(!end.cancelled && !end.timed_out);
        assert!(out.contains(&"out:ok sem".to_owned()));
        assert!(out.contains(&"err:erro".to_owned()));
    }

    #[test]
    fn reports_exit_status_and_cancellation() {
        let registry = Registry::default();
        registry.register(2);
        let end = run(plan("exit 3"), 2, &registry, |_| {}).unwrap();
        assert_eq!(end.status, Some(3));

        registry.register(3);
        assert!(registry.cancel(3));
        assert!(!registry.cancel(99));
        let started = Instant::now();
        let end = run(plan("sleep 30"), 3, &registry, |_| {}).unwrap();
        assert!(end.cancelled);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
