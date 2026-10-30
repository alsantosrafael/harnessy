//! Conexão SQLite compartilhada e evolução do esquema.
//!
//! Uma única conexão protegida por `Mutex` é suficiente para o volume local do
//! aplicativo. Os módulos de domínio recebem `&Connection` e não conhecem o
//! caminho do arquivo nem a estratégia de bloqueio.

use rusqlite::Connection;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use crate::error::{AppError, Result};

/// Esquemas aplicados em ordem; `PRAGMA user_version` guarda quantos já rodaram.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE workspaces (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        folder TEXT NOT NULL UNIQUE
    );
    CREATE TABLE tasks (
        id INTEGER PRIMARY KEY,
        workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
        title TEXT NOT NULL,
        description TEXT NOT NULL,
        criteria TEXT NOT NULL,
        revision INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX tasks_workspace ON tasks(workspace_id);
"];

pub struct Database(Mutex<Connection>);

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(3))?;
        db.execute_batch("PRAGMA foreign_keys=ON;")?;
        migrate(&db)?;
        Ok(Self(Mutex::new(db)))
    }

    pub fn connection(&self) -> Result<MutexGuard<'_, Connection>> {
        self.0.lock().map_err(|_| AppError::DatabaseUnavailable)
    }
}

fn migrate(db: &Connection) -> Result<()> {
    let current: usize = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > MIGRATIONS.len() {
        return Err(AppError::SchemaTooNew);
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        db.execute_batch(&format!(
            "BEGIN IMMEDIATE; {sql} PRAGMA user_version={}; COMMIT;",
            index + 1
        ))?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// Banco temporário apagado ao sair de escopo.
    pub struct TempDatabase {
        pub path: std::path::PathBuf,
    }

    impl TempDatabase {
        pub fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "harnessy-test-{label}-{}.sqlite",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            Self { path }
        }

        pub fn open(&self) -> Database {
            Database::open(&self.path).expect("abrir banco de teste")
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent_and_reject_newer_schema() {
        let temp = test_support::TempDatabase::new("db");
        drop(temp.open());
        let reopened = temp.open();
        let db = reopened.connection().unwrap();
        let version: usize = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len());
        db.execute_batch("PRAGMA user_version=99;").unwrap();
        assert!(matches!(migrate(&db), Err(AppError::SchemaTooNew)));
    }
}
