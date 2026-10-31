//! Ambiente de teste: junta perfil do workspace, aprovação do usuário e o worktree da run.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::{
    error::{AppError, Result},
    process, profile,
    run::{
        self, guard,
        preview::{self, Hooks, Spec},
        worktree,
    },
    workspace,
};

/// O que a interface precisa para oferecer "Rodar localmente".
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    /// O arquivo `.harnessy/workspace.toml` existe e é válido.
    pub exists: bool,
    pub run: Option<String>,
    pub port: Option<u16>,
    /// O usuário já confirmou o texto atual do arquivo.
    pub approved: bool,
    /// Comando sugerido a partir do `package.json` quando não há perfil.
    pub suggestion: Option<String>,
    /// Frase curta quando o arquivo existe mas não presta.
    pub error: Option<String>,
}

pub fn profile_view(db: &Connection, workspace_id: i64) -> Result<ProfileView> {
    let ws = workspace::get(db, workspace_id)?;
    let dir = Path::new(&ws.folder);
    match profile::read(dir) {
        Ok(Some(loaded)) => Ok(ProfileView {
            exists: true,
            run: Some(loaded.profile.preview.run.clone()),
            port: loaded.profile.preview.port,
            approved: profile::is_approved(db, workspace_id, &loaded.sha256)?,
            suggestion: None,
            error: None,
        }),
        Ok(None) => Ok(ProfileView {
            exists: false,
            run: None,
            port: None,
            approved: false,
            suggestion: suggestion(dir),
            error: None,
        }),
        Err(error) => Ok(ProfileView {
            exists: false,
            run: None,
            port: None,
            approved: false,
            suggestion: None,
            error: Some(error.to_string()),
        }),
    }
}

fn suggestion(dir: &Path) -> Option<String> {
    let package = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let bun = dir.join("bun.lock").exists() || dir.join("bun.lockb").exists();
    profile::suggest(&package, bun)
}

/// O usuário escolheu o comando no formulário: grava o arquivo e já o considera confirmado.
pub fn save_profile(
    db: &Connection,
    workspace_id: i64,
    run: &str,
    port: Option<u16>,
) -> Result<ProfileView> {
    let ws = workspace::get(db, workspace_id)?;
    let sha = profile::write(Path::new(&ws.folder), run, port)?;
    profile::approve(db, workspace_id, &sha)?;
    profile_view(db, workspace_id)
}

/// O usuário viu o comando do arquivo do repositório e confirmou.
pub fn approve_profile(db: &Connection, workspace_id: i64) -> Result<ProfileView> {
    let ws = workspace::get(db, workspace_id)?;
    let loaded = profile::read(Path::new(&ws.folder))?.ok_or(AppError::Validation(
        "Este workspace não tem .harnessy/workspace.toml.",
    ))?;
    profile::approve(db, workspace_id, &loaded.sha256)?;
    profile_view(db, workspace_id)
}

/// Plano de execução do ambiente de uma run, ou a frase do que falta.
///
/// O worktree de uma run é removido quando ela termina; a branch fica. Se a pasta sumiu,
/// o plano traz os passos para recriá-la a partir da branch (e removê-la ao fim).
pub fn plan_for(
    db: &Connection,
    run_id: i64,
    dirs: &[PathBuf],
    cargo_target: &Path,
) -> Result<(Spec, Hooks)> {
    let run = run::get(db, run_id)?;
    let ws = workspace::get(db, run.workspace_id)?;
    let loaded = profile::read(Path::new(&ws.folder))?.ok_or(AppError::Validation(
        "Defina como subir o projeto antes de rodar localmente.",
    ))?;
    if !profile::is_approved(db, run.workspace_id, &loaded.sha256)? {
        return Err(AppError::Validation(
            "Confirme o comando do projeto antes de rodá-lo.",
        ));
    }
    let dir = run
        .worktree_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or(AppError::Validation(
            "Esta run não deixou uma cópia para rodar.",
        ))?;
    // Porta: com `{port}` no comando o Harnessy escolhe uma livre; sem ele, a declarada tem
    // de estar livre (e isso é conferido antes de gastar minutos recriando a cópia).
    let declared = loaded.profile.preview.port;
    let auto = loaded
        .profile
        .preview
        .run
        .contains(profile::PORT_PLACEHOLDER);
    if !auto {
        if let Some(port) = declared.filter(|port| preview::port_in_use(*port)) {
            return Err(AppError::Delivery(format!(
                "A porta {port} já está em uso por outro programa. Escreva {} no comando, no lugar do número, e o Harnessy escolhe uma livre.",
                profile::PORT_PLACEHOLDER
            )));
        }
    }
    let port = if auto {
        Some(preview::free_port(declared).ok_or(AppError::Validation(
            "Não encontrei uma porta livre neste computador.",
        ))?)
    } else {
        declared
    };
    let command = match port {
        Some(port) if auto => loaded
            .profile
            .preview
            .run
            .replace(profile::PORT_PLACEHOLDER, &port.to_string()),
        _ => loaded.profile.preview.run.clone(),
    };
    let hooks = if dir.is_dir() {
        Hooks::default()
    } else {
        recreate(&ws.folder, &dir, run.branch.as_deref(), dirs)?
    };
    let spec = Spec {
        run_id,
        command,
        cwd: dir,
        env: guard::env(dirs, cargo_target, |name| std::env::var(name).ok()),
        port,
        ready: loaded.profile.preview.ready,
    };
    Ok((spec, hooks))
}

/// Passos para recriar a cópia da run a partir da branch dela, com dependências instaladas.
fn recreate(repo: &str, dir: &Path, branch: Option<&str>, dirs: &[PathBuf]) -> Result<Hooks> {
    let branch = branch
        .ok_or(AppError::Validation(
            "A run não deixou uma branch para rodar.",
        ))?
        .to_owned();
    let git = process::find_binary("git", dirs)
        .ok_or(AppError::Validation("git não encontrado neste computador."))?;
    let bun = process::find_binary("bun", dirs);
    let (repo, dir_prepare, dir_cleanup) =
        (PathBuf::from(repo), dir.to_path_buf(), dir.to_path_buf());
    let (dirs_prepare, dirs_cleanup) = (dirs.to_vec(), dirs.to_vec());
    let (git_cleanup, repo_cleanup) = (git.clone(), repo.clone());
    Ok(Hooks {
        prepare: Some(Box::new(move |log| {
            log("A cópia desta run não existe mais; recriando a partir da branch…");
            worktree::checkout(&git, &dirs_prepare, &repo, &dir_prepare, &branch)
                .map_err(AppError::Delivery)?;
            log("Instalando as dependências…");
            worktree::install_deps(bun.as_deref(), &dirs_prepare, &dir_prepare)
                .map_err(AppError::Delivery)?;
            log("Cópia pronta; subindo o projeto…");
            Ok(())
        })),
        cleanup: Some(Box::new(move || {
            let _ = worktree::remove(&git_cleanup, &dirs_cleanup, &repo_cleanup, &dir_cleanup);
        })),
    })
}
