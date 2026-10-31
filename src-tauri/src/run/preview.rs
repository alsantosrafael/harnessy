//! Ambiente de teste local: sobe o projeto da run para o humano ver e testar a mudança.
//!
//! É o primeiro "alvo" de ambiente; um deploy em QA entra no mesmo lugar depois.
//! O comando vem do perfil do workspace já confirmado pelo usuário e roda no
//! worktree da run, com o ambiente fixo de [`super::guard::env`] (sem `GH_TOKEN`)
//! e em grupo de processos próprio, para que parar encerre tudo o que ele abriu.

use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Serialize;

use super::redact;
use crate::{
    db::now,
    error::{AppError, Result},
};

const MAX_LOG_LINES: usize = 500;
const MAX_LINE_CHARS: usize = 400;
const PORT_POLL: Duration = Duration::from_millis(400);
const PORT_GIVE_UP: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub run_id: i64,
    /// `starting`, `running`, `stopped` ou `failed`.
    pub state: &'static str,
    /// Só endereços locais (`http://localhost:N`): é o que o botão Abrir usa.
    pub url: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at: i64,
}

/// O que mudou, para a interface acompanhar sem perguntar.
#[derive(Debug, Clone)]
pub enum Event {
    State(Info),
    Log { run_id: i64, line: String },
}

pub struct Spec {
    pub run_id: i64,
    pub command: String,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub port: Option<u16>,
    pub ready: Option<String>,
}

struct Entry {
    info: Info,
    pgid: Option<i32>,
    stopping: bool,
    log: VecDeque<String>,
}

#[derive(Clone, Default)]
pub struct Previews {
    inner: Arc<Mutex<HashMap<i64, Entry>>>,
}

fn alive(state: &str) -> bool {
    matches!(state, "starting" | "running")
}

fn kill_group(pgid: i32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .status();
}

impl Previews {
    fn with<T>(&self, run_id: i64, f: impl FnOnce(&mut Entry) -> T) -> Option<T> {
        self.inner.lock().ok()?.get_mut(&run_id).map(f)
    }

    pub fn info(&self, run_id: i64) -> Option<Info> {
        self.with(run_id, |entry| entry.info.clone())
    }

    pub fn log(&self, run_id: i64) -> Vec<String> {
        self.with(run_id, |entry| entry.log.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Ambientes vivos de todas as runs (no máximo um, por ora).
    pub fn active(&self) -> Vec<Info> {
        self.inner
            .lock()
            .map(|map| {
                map.values()
                    .filter(|entry| alive(entry.info.state))
                    .map(|entry| entry.info.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Encerra o ambiente de uma run e tudo o que ele abriu.
    pub fn stop(&self, run_id: i64) -> bool {
        match self.with(run_id, |entry| {
            entry.stopping = true;
            entry.pgid
        }) {
            Some(Some(pgid)) => {
                kill_group(pgid);
                true
            }
            _ => false,
        }
    }

    pub fn stop_all(&self) {
        let ids: Vec<i64> = self.active().iter().map(|info| info.run_id).collect();
        for id in ids {
            self.stop(id);
        }
    }

    fn set(
        &self,
        run_id: i64,
        state: &'static str,
        url: Option<String>,
        code: Option<i32>,
    ) -> Option<Info> {
        self.with(run_id, |entry| {
            entry.info.state = state;
            if url.is_some() {
                entry.info.url = url;
            }
            entry.info.exit_code = code;
            entry.info.clone()
        })
    }

    /// Sobe o ambiente. Só um fica vivo por vez (portas e memória): o anterior é encerrado.
    pub fn start(
        &self,
        spec: Spec,
        notify: impl Fn(Event) + Send + Sync + 'static,
    ) -> Result<Info> {
        if self.info(spec.run_id).is_some_and(|info| alive(info.state)) {
            return Err(AppError::Delivery(
                "O ambiente desta run já está no ar.".into(),
            ));
        }
        self.stop_all();
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", &spec.command])
            .env_clear()
            .envs(spec.env.iter().map(|(key, value)| (key, value)))
            .current_dir(&spec.cwd)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(port) = spec.port {
            command.env("PORT", port.to_string());
        }
        let mut child = command.spawn().map_err(|_| {
            AppError::Delivery("Não foi possível iniciar o ambiente de teste.".into())
        })?;
        let info = Info {
            run_id: spec.run_id,
            state: "starting",
            url: None,
            exit_code: None,
            started_at: now(),
        };
        if let Ok(mut map) = self.inner.lock() {
            map.insert(
                spec.run_id,
                Entry {
                    info: info.clone(),
                    pgid: Some(child.id() as i32),
                    stopping: false,
                    log: VecDeque::new(),
                },
            );
        }
        let notify = Arc::new(notify);
        let run_id = spec.run_id;
        for stream in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let (previews, notify, ready) = (self.clone(), notify.clone(), spec.ready.clone());
            std::thread::spawn(move || {
                for raw in BufReader::new(stream)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    previews.line(run_id, &raw, ready.as_deref(), &*notify);
                }
            });
        }
        if let Some(port) = spec.port {
            let (previews, notify) = (self.clone(), notify.clone());
            std::thread::spawn(move || previews.wait_for_port(run_id, port, &*notify));
        }
        let (previews, notify) = (self.clone(), notify);
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|exit| exit.code());
            let stopped = previews
                .with(run_id, |entry| entry.stopping)
                .unwrap_or(false);
            let state = if stopped || code == Some(0) {
                "stopped"
            } else {
                "failed"
            };
            if let Some(info) = previews.set(run_id, state, None, code) {
                notify(Event::State(info));
            }
        });
        Ok(info)
    }

    fn line(&self, run_id: i64, raw: &str, ready: Option<&str>, notify: &dyn Fn(Event)) {
        let plain = strip_ansi(raw);
        let line: String = redact::clean(&plain).chars().take(MAX_LINE_CHARS).collect();
        self.with(run_id, |entry| {
            if entry.log.len() == MAX_LOG_LINES {
                entry.log.pop_front();
            }
            entry.log.push_back(line.clone());
        });
        notify(Event::Log { run_id, line });
        let starting = self
            .info(run_id)
            .is_some_and(|info| info.state == "starting");
        if !starting {
            return;
        }
        let url = find_url(&plain);
        let matched = ready.is_some_and(|text| plain.contains(text));
        if url.is_some() || matched {
            if let Some(info) = self.set(run_id, "running", url, None) {
                notify(Event::State(info));
            }
        }
    }

    fn wait_for_port(&self, run_id: i64, port: u16, notify: &dyn Fn(Event)) {
        let started = Instant::now();
        let targets: [SocketAddr; 2] = [
            SocketAddr::from(([127, 0, 0, 1], port)),
            SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)),
        ];
        while started.elapsed() < PORT_GIVE_UP {
            if !self.info(run_id).is_some_and(|info| alive(info.state)) {
                return;
            }
            if targets
                .iter()
                .any(|addr| TcpStream::connect_timeout(addr, Duration::from_millis(200)).is_ok())
            {
                if let Some(info) = self.set(
                    run_id,
                    "running",
                    Some(format!("http://localhost:{port}")),
                    None,
                ) {
                    notify(Event::State(info));
                }
                return;
            }
            std::thread::sleep(PORT_POLL);
        }
    }

    /// Abre no navegador padrão, só um endereço local que o próprio ambiente informou.
    pub fn open(&self, run_id: i64) -> Result<()> {
        let url = self
            .info(run_id)
            .filter(|info| info.state == "running")
            .and_then(|info| info.url)
            .filter(|url| is_local(url))
            .ok_or(AppError::Validation("O ambiente ainda não está no ar."))?;
        Command::new("/usr/bin/open")
            .arg(url)
            .status()
            .map_err(|_| AppError::Delivery("Não foi possível abrir o navegador.".into()))?;
        Ok(())
    }
}

fn is_local(url: &str) -> bool {
    url.strip_prefix("http://localhost:")
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

/// Remove sequências de cor ANSI (`ESC [ ... letra`), que Vite e afins imprimem.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Primeiro endereço local que a linha anuncia, normalizado para `http://localhost:N`.
pub fn find_url(line: &str) -> Option<String> {
    for host in ["localhost", "127.0.0.1", "0.0.0.0", "[::1]"] {
        let marker = format!("http://{host}:");
        if let Some(at) = line.find(&marker) {
            let digits: String = line[at + marker.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if !digits.is_empty() && digits.len() <= 5 {
                return Some(format!("http://localhost:{digits}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn finds_local_urls_in_colored_dev_server_output() {
        let line = "\u{1b}[32m  ➜  Local:\u{1b}[0m   http://localhost:\u{1b}[1m5173\u{1b}[0m/";
        assert_eq!(
            find_url(&strip_ansi(line)).as_deref(),
            Some("http://localhost:5173")
        );
        assert_eq!(
            find_url("ready on http://127.0.0.1:3000").as_deref(),
            Some("http://localhost:3000")
        );
        assert_eq!(find_url("https://example.com:443"), None);
        assert_eq!(find_url("http://localhost:/x"), None);
    }

    #[test]
    fn only_local_urls_can_be_opened() {
        assert!(is_local("http://localhost:5173"));
        assert!(!is_local("http://evil.test:5173"));
        assert!(!is_local("http://localhost:5173/../x"));
        assert!(!is_local("file:///etc/passwd"));
    }

    fn spec(command: &str) -> Spec {
        Spec {
            run_id: 1,
            command: command.into(),
            cwd: std::env::temp_dir(),
            env: vec![("PATH".into(), "/usr/bin:/bin".into())],
            port: None,
            ready: None,
        }
    }

    fn wait_for(previews: &Previews, state: &str) -> Info {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(info) = previews.info(1).filter(|info| info.state == state) {
                return info;
            }
            assert!(
                Instant::now() < until,
                "esperava {state}: {:?}",
                previews.info(1)
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_dev_server_goes_running_with_its_url_then_stops_with_its_children() {
        let previews = Previews::default();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        previews
            .start(
                spec("echo 'Local: http://localhost:4321'; sleep 30"),
                move |event| {
                    if let Event::Log { line, .. } = event {
                        let _ = tx.lock().map(|tx| tx.send(line));
                    }
                },
            )
            .unwrap();
        let running = wait_for(&previews, "running");
        assert_eq!(running.url.as_deref(), Some("http://localhost:4321"));
        assert!(rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .contains("4321"));
        assert!(previews.log(1).iter().any(|line| line.contains("Local:")));
        assert!(
            previews.start(spec("sleep 1"), |_| {}).is_err(),
            "já está no ar"
        );
        assert!(previews.stop(1));
        assert_eq!(
            wait_for(&previews, "stopped").url.as_deref(),
            Some("http://localhost:4321")
        );
    }

    #[test]
    fn a_command_that_dies_is_reported_as_failed_with_its_exit_code() {
        let previews = Previews::default();
        previews.start(spec("echo erro; exit 3"), |_| {}).unwrap();
        assert_eq!(wait_for(&previews, "failed").exit_code, Some(3));
    }

    #[test]
    fn the_log_keeps_only_the_latest_lines() {
        let previews = Previews::default();
        previews
            .start(
                spec("i=0; while [ $i -lt 700 ]; do echo l$i; i=$((i+1)); done"),
                |_| {},
            )
            .unwrap();
        wait_for(&previews, "stopped");
        let log = previews.log(1);
        assert_eq!(log.len(), MAX_LOG_LINES);
        assert_eq!(log.last().map(String::as_str), Some("l699"));
    }
}
