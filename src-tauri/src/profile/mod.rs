//! Perfil do workspace: como subir o projeto para quem quer ver a mudança funcionando.
//!
//! Vive em `.harnessy/workspace.toml`, no repositório do usuário. O comando dele
//! roda na máquina de quem clica, então só roda depois de o usuário confirmar o
//! texto exato (aprovação por sha256, como nas Ações do repositório). Hoje o alvo é
//! `preview` (rodar local); o mesmo perfil vai receber `qa` quando houver deploy.

use std::{fs, path::Path};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    db::now,
    error::{AppError, Result},
};

pub const PATH: &str = ".harnessy/workspace.toml";
const SCHEMA: u32 = 1;
const MAX_COMMAND_CHARS: usize = 300;
const MAX_READY_CHARS: usize = 80;
const MAX_FILE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    /// Comando que sobe o projeto, executado na raiz do worktree da run.
    pub run: String,
    /// Porta que o projeto abre; sem ela, a URL vem do que o comando imprime.
    #[serde(default)]
    pub port: Option<u16>,
    /// Trecho da saída que indica que subiu (opcional).
    #[serde(default)]
    pub ready: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: u32,
    pub preview: Preview,
}

/// Perfil lido do disco com o hash do texto exato, para a confirmação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub profile: Profile,
    pub sha256: String,
}

fn invalid(reason: &str) -> AppError {
    AppError::Delivery(format!("{PATH} inválido: {reason}"))
}

pub fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn parse(text: &str) -> Result<Profile> {
    let profile: Profile = toml::from_str(text).map_err(|_| invalid("formato do TOML"))?;
    if profile.schema != SCHEMA {
        return Err(invalid("schema deve ser 1"));
    }
    let run = profile.preview.run.trim();
    if run.is_empty() || run.chars().count() > MAX_COMMAND_CHARS || run.contains(['\n', '\r']) {
        return Err(invalid(
            "preview.run deve ter uma linha de até 300 caracteres",
        ));
    }
    if profile.preview.port.is_some_and(|port| port < 1024) {
        return Err(invalid("preview.port deve ser 1024 ou mais"));
    }
    if profile
        .preview
        .ready
        .as_deref()
        .is_some_and(|ready| ready.chars().count() > MAX_READY_CHARS || ready.contains('\n'))
    {
        return Err(invalid("preview.ready deve ter uma linha curta"));
    }
    Ok(profile)
}

/// Texto do arquivo para um comando e porta escolhidos.
pub fn render(run: &str, port: Option<u16>) -> Result<String> {
    let profile = Profile {
        schema: SCHEMA,
        preview: Preview {
            run: run.trim().to_owned(),
            port,
            ready: None,
        },
    };
    let text =
        toml::to_string(&profile).map_err(|_| invalid("não foi possível gerar o arquivo"))?;
    parse(&text)?;
    Ok(text)
}

/// Lê o perfil da pasta do workspace; `None` se o arquivo não existe.
pub fn read(dir: &Path) -> Result<Option<Loaded>> {
    let path = dir.join(PATH);
    let Ok(meta) = fs::metadata(&path) else {
        return Ok(None);
    };
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return Err(invalid("arquivo grande demais ou não é um arquivo"));
    }
    let text = fs::read_to_string(&path).map_err(|_| invalid("não foi possível ler"))?;
    Ok(Some(Loaded {
        profile: parse(&text)?,
        sha256: sha256(&text),
    }))
}

/// Grava o perfil escolhido pelo usuário e devolve o hash do que foi escrito.
pub fn write(dir: &Path, run: &str, port: Option<u16>) -> Result<String> {
    let text = render(run, port)?;
    let path = dir.join(PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| invalid("não foi possível criar .harnessy"))?;
    }
    fs::write(&path, &text).map_err(|_| invalid("não foi possível gravar"))?;
    Ok(sha256(&text))
}

/// Comando sugerido a partir do `package.json` (script `dev`, senão `start`).
pub fn suggest(package_json: &str, uses_bun: bool) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(package_json).ok()?;
    let scripts = value.get("scripts")?;
    let name = ["dev", "start"]
        .into_iter()
        .find(|name| scripts.get(*name).is_some_and(|script| script.is_string()))?;
    Some(format!(
        "{} run {name}",
        if uses_bun { "bun" } else { "npm" }
    ))
}

/// Registra que o usuário confirmou este texto exato para o workspace.
pub fn approve(db: &Connection, workspace_id: i64, sha256: &str) -> Result<()> {
    db.execute(
        "INSERT INTO profile_approvals (workspace_id, sha256, approved_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(workspace_id) DO UPDATE SET sha256=?2, approved_at=?3",
        params![workspace_id, sha256, now()],
    )?;
    Ok(())
}

/// O texto atual é o mesmo que o usuário confirmou; qualquer edição pede nova confirmação.
pub fn is_approved(db: &Connection, workspace_id: i64, sha256: &str) -> Result<bool> {
    let saved: Option<String> = db
        .query_row(
            "SELECT sha256 FROM profile_approvals WHERE workspace_id=?1",
            [workspace_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(saved.as_deref() == Some(sha256))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    const FILE: &str = "schema = 1\n[preview]\nrun = \"bun run dev\"\nport = 5173\n";

    #[test]
    fn parses_a_valid_profile() {
        let profile = parse(FILE).unwrap();
        assert_eq!(profile.preview.run, "bun run dev");
        assert_eq!(profile.preview.port, Some(5173));
    }

    #[test]
    fn rejects_bad_schema_multiline_commands_low_ports_and_unknown_keys() {
        for bad in [
            "schema = 2\n[preview]\nrun = \"x\"\n",
            "schema = 1\n[preview]\nrun = \"a\\nb\"\n",
            "schema = 1\n[preview]\nrun = \"  \"\n",
            "schema = 1\n[preview]\nrun = \"x\"\nport = 80\n",
            "schema = 1\n[preview]\nrun = \"x\"\nenv = \"A=1\"\n",
            "não é toml",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn written_files_round_trip_and_hash_the_exact_text() {
        let dir = std::env::temp_dir().join(format!("harnessy-profile-{}", now()));
        let sha = write(&dir, " bun run dev ", Some(3000)).unwrap();
        let loaded = read(&dir).unwrap().unwrap();
        assert_eq!(loaded.sha256, sha);
        assert_eq!(loaded.profile.preview.run, "bun run dev");
        assert!(read(&dir.join("nada")).unwrap().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn suggests_dev_before_start_and_the_right_runner() {
        let both = r#"{"scripts":{"start":"x","dev":"y"}}"#;
        assert_eq!(suggest(both, true).as_deref(), Some("bun run dev"));
        assert_eq!(
            suggest(r#"{"scripts":{"start":"x"}}"#, false).as_deref(),
            Some("npm run start")
        );
        assert_eq!(suggest(r#"{"scripts":{"test":"x"}}"#, true), None);
        assert_eq!(suggest("lixo", true), None);
    }

    #[test]
    fn approval_is_tied_to_the_exact_text() {
        let temp = TempDatabase::new("profile");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute("PRAGMA foreign_keys=OFF", []).unwrap();
        assert!(!is_approved(&db, 1, "aaa").unwrap());
        approve(&db, 1, "aaa").unwrap();
        assert!(is_approved(&db, 1, "aaa").unwrap());
        assert!(
            !is_approved(&db, 1, "bbb").unwrap(),
            "arquivo editado pede nova confirmação"
        );
    }
}
