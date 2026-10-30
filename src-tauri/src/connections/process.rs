//! Localização e execução controlada de CLIs externas.

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const OUTPUT_LIMIT: u64 = 16 * 1024;

/// Saída combinada (stdout seguido de stderr) e código de término.
pub struct Output {
    pub status: Option<i32>,
    pub text: String,
}

impl Output {
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }
}

/// Diretórios onde CLIs costumam ficar. Aplicativos GUI no macOS não herdam o
/// PATH do shell, então o PATH do processo é apenas um dos candidatos.
pub fn search_dirs() -> Vec<PathBuf> {
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

pub fn find_binary(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Executa a CLI com tempo limite. Entrada fechada: a CLI não pode abrir um
/// fluxo interativo de login a partir daqui.
pub fn run(binary: &Path, args: &[&str], dirs: &[PathBuf]) -> Result<Output, String> {
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
            let _ = stream.take(OUTPUT_LIMIT).read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    });

    let status = wait_with_timeout(&mut child)?;
    let text = reader.join().unwrap_or_default();
    Ok(Output { status, text })
}

fn wait_with_timeout(child: &mut std::process::Child) -> Result<Option<i32>, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.code()),
            Ok(None) if started.elapsed() < TIMEOUT => std::thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("a verificação demorou demais e foi interrompida".into());
            }
            Err(error) => return Err(format!("falha ao aguardar a CLI: {error}")),
        }
    }
}
