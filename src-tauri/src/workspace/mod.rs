//! Workspaces: pastas locais vinculadas a um nome.
//!
//! Regras de validação e acesso ao banco vivem aqui, sobre `&Connection`, para
//! serem testáveis sem Tauri. Os comandos IPC ficam em [`commands`].

pub mod commands;

use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;

use crate::error::{AppError, Result};

const MAX_NAME_BYTES: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub folder: String,
}

pub fn list(db: &Connection) -> Result<Vec<Workspace>> {
    let mut query = db.prepare("SELECT id, name, folder FROM workspaces ORDER BY id")?;
    let rows = query.query_map([], |r| {
        Ok(Workspace {
            id: r.get(0)?,
            name: r.get(1)?,
            folder: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn create(db: &Connection, name: &str, folder: &str) -> Result<Workspace> {
    let name = validate_name(name)?;
    let folder = canonical_folder(folder)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM workspaces WHERE folder=?1)",
        [&folder],
        |r| r.get(0),
    )?;
    if exists {
        return Err(AppError::Validation(
            "Esta pasta já está vinculada a um workspace.",
        ));
    }
    db.execute(
        "INSERT INTO workspaces(name, folder) VALUES (?1, ?2)",
        params![name, folder],
    )?;
    Ok(Workspace {
        id: db.last_insert_rowid(),
        name: name.to_owned(),
        folder,
    })
}

fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(AppError::Validation("Informe um nome de até 200 bytes."));
    }
    Ok(name)
}

/// Aceita apenas pastas absolutas existentes e devolve o caminho canônico.
fn canonical_folder(folder: &str) -> Result<String> {
    let path = Path::new(folder.trim());
    if !path.is_absolute() {
        return Err(AppError::Validation(
            "Informe o caminho absoluto de uma pasta local.",
        ));
    }
    let path = path
        .canonicalize()
        .map_err(|_| AppError::Validation("Pasta não encontrada ou sem permissão de acesso."))?;
    if !path.is_dir() {
        return Err(AppError::Validation(
            "O caminho deve apontar para uma pasta.",
        ));
    }
    path.to_str().map(str::to_owned).ok_or(AppError::Validation(
        "O caminho precisa conter texto UTF-8 válido.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    #[test]
    fn creates_once_per_folder_and_validates_input() {
        let temp = TempDatabase::new("workspace");
        let store = temp.open();
        let db = store.connection().unwrap();
        let folder = std::env::temp_dir();
        let folder = folder.to_str().unwrap();

        let created = create(&db, "  Piloto ", folder).unwrap();
        assert_eq!(created.name, "Piloto");
        assert!(matches!(
            create(&db, "Outro", folder),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            create(&db, " ", folder),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            create(&db, "Relativo", "pasta/relativa"),
            Err(AppError::Validation(_))
        ));
        assert_eq!(list(&db).unwrap().len(), 1);
    }
}
