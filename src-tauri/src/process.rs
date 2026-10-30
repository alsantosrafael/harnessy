//! Localização e execução controlada de CLIs externas de curta duração.
//!
//! Infraestrutura compartilhada: sondas de conexão, leitura de Issues e comandos
//! git passam por aqui, sempre com `stdin` fechado, tempo limite e saída
//! limitada. Processos de longa duração dos agentes ficam em `run::supervisor`.

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const OUTPUT_LIMIT: usize = 16 * 1024;
/// Shell de login que carrega nvm, asdf e afins pode demorar; passado disso, usa o PATH comum.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(8);
const PATH_START: &str = "__HARNESSY_PATH__";
const PATH_END: &str = "__HARNESSY_END__";

/// PATH do shell de login do usuário, lido uma vez. Fica vazio se a leitura falhar.
static LOGIN_PATH: OnceLock<Vec<PathBuf>> = OnceLock::new();

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

/// Ajustes de uma execução: diretório, tempo máximo e bytes de saída mantidos.
pub struct Options<'a> {
    pub cwd: Option<&'a Path>,
    pub timeout: Duration,
    pub limit: usize,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            cwd: None,
            timeout: TIMEOUT,
            limit: OUTPUT_LIMIT,
        }
    }
}

/// Diretórios onde CLIs costumam ficar. Aplicativos GUI no macOS não herdam o
/// PATH do shell: o do shell de login (já lido, se pronto) vem primeiro, para que
/// `node` seja o do nvm e não o `/usr/local/bin` antigo; depois o PATH do processo.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = LOGIN_PATH.get().cloned().unwrap_or_default();
    dirs.extend(base_dirs());
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|dir| seen.insert(dir.clone()));
    dirs
}

/// Começa a ler o PATH do shell de login em segundo plano (chamar na abertura do app).
pub fn warm_up() {
    std::thread::spawn(|| {
        LOGIN_PATH.get_or_init(read_login_path);
    });
}

/// Espera a leitura do PATH de login terminar; antes de iniciar uma run, para ela
/// usar as mesmas ferramentas do terminal do usuário.
pub fn wait_for_login_path() {
    LOGIN_PATH.get_or_init(read_login_path);
}

fn read_login_path() -> Vec<PathBuf> {
    let shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
    // `-i -l`: o nvm costuma registrar o Node no .zshrc, que só o shell interativo lê.
    let script = format!("printf '{PATH_START}%s{PATH_END}' \"$PATH\"");
    let options = Options {
        cwd: None,
        timeout: LOGIN_TIMEOUT,
        limit: OUTPUT_LIMIT,
    };
    run_with(&shell, &["-ilc", &script], &base_dirs(), &options)
        .map(|output| parse_login_path(&output.text))
        .unwrap_or_default()
}

/// Extrai o PATH entre os marcadores, ignorando o ruído que o rc do shell imprimir.
fn parse_login_path(text: &str) -> Vec<PathBuf> {
    let Some(start) = text.rfind(PATH_START) else {
        return Vec::new();
    };
    let rest = &text[start + PATH_START.len()..];
    let Some(end) = rest.find(PATH_END) else {
        return Vec::new();
    };
    std::env::split_paths(&rest[..end])
        .filter(|dir| dir.is_absolute())
        .collect()
}

fn base_dirs() -> Vec<PathBuf> {
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

/// Executa a CLI com os limites padrão. Entrada fechada: a CLI não pode abrir
/// um fluxo interativo de login a partir daqui.
pub fn run(binary: &Path, args: &[&str], dirs: &[PathBuf]) -> Result<Output, String> {
    run_with(binary, args, dirs, &Options::default())
}

pub fn run_with(
    binary: &Path,
    args: &[&str],
    dirs: &[PathBuf],
    options: &Options<'_>,
) -> Result<Output, String> {
    let path = std::env::join_paths(dirs).map_err(|error| error.to_string())?;
    let mut command = Command::new(binary);
    command
        .args(args)
        .env("PATH", path)
        .env("NO_COLOR", "1")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = options.cwd {
        command.current_dir(cwd);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("não foi possível executar a CLI: {error}"))?;

    let limit = options.limit;
    let stdout = child
        .stdout
        .take()
        .map(|s| std::thread::spawn(move || drain(s, limit)));
    let stderr = child
        .stderr
        .take()
        .map(|s| std::thread::spawn(move || drain(s, limit)));

    let status = wait_with_timeout(&mut child, options.timeout)?;
    let mut text = Vec::new();
    for reader in [stdout, stderr].into_iter().flatten() {
        text.extend(reader.join().unwrap_or_default());
    }
    Ok(Output {
        status,
        text: String::from_utf8_lossy(&text).into_owned(),
    })
}

/// Lê o fluxo até o fim, guardando só os primeiros `limit` bytes. Continuar a
/// ler evita que a CLI bloqueie ao encher o pipe.
fn drain(mut stream: impl Read, limit: usize) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return kept,
            Ok(n) => {
                let room = limit.saturating_sub(kept.len());
                kept.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<Option<i32>, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.code()),
            Ok(None) if started.elapsed() < timeout => std::thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("o comando demorou demais e foi interrompido".into());
            }
            Err(error) => return Err(format!("falha ao aguardar a CLI: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_keeps_only_the_limit() {
        let data = vec![b'a'; 20_000];
        assert_eq!(drain(&data[..], 100).len(), 100);
    }

    #[test]
    fn runs_in_given_directory_and_times_out() {
        let dirs = search_dirs();
        let pwd = find_binary("pwd", &dirs).expect("pwd");
        let dir = std::env::temp_dir().canonicalize().unwrap();
        let options = Options {
            cwd: Some(&dir),
            ..Options::default()
        };
        let out = run_with(&pwd, &[], &dirs, &options).unwrap();
        assert!(out.succeeded());
        assert_eq!(out.text.trim(), dir.to_str().unwrap());

        let sleep = find_binary("sleep", &dirs).expect("sleep");
        let options = Options {
            timeout: Duration::from_millis(200),
            ..Options::default()
        };
        assert!(run_with(&sleep, &["5"], &dirs, &options).is_err());
    }

    #[test]
    fn login_path_is_read_between_markers_and_ignores_rc_noise() {
        let text = format!(
            "Last login: hoje\nnvm: carregado\n{PATH_START}/Users/x/.nvm/v22/bin:/usr/local/bin:relativo{PATH_END}\naviso do shell"
        );
        assert_eq!(
            parse_login_path(&text),
            vec![
                PathBuf::from("/Users/x/.nvm/v22/bin"),
                PathBuf::from("/usr/local/bin")
            ]
        );
        assert!(parse_login_path("sem marcadores").is_empty());
        assert!(parse_login_path(&format!("{PATH_START}/incompleto")).is_empty());
    }

    #[test]
    fn the_login_shell_path_comes_first_and_without_duplicates() {
        let dirs = search_dirs();
        let unique: std::collections::HashSet<_> = dirs.iter().collect();
        assert_eq!(unique.len(), dirs.len());
        assert!(dirs.iter().any(|dir| dir == Path::new("/usr/bin")));
    }
}
