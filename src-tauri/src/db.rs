//! Conexão SQLite compartilhada e evolução do esquema.
//!
//! Uma única conexão protegida por `Mutex` é suficiente para o volume local do
//! aplicativo. Os módulos de domínio recebem `&Connection` e não conhecem o
//! caminho do arquivo nem a estratégia de bloqueio.

use rusqlite::Connection;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::error::{AppError, Result};

/// Esquemas aplicados em ordem; `PRAGMA user_version` guarda quantos já rodaram.
const MIGRATIONS: &[&str] = &[
    "
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
        revision INTEGER NOT NULL DEFAULT 1,
        status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo', 'doing', 'review', 'done'))
    );
    CREATE INDEX tasks_workspace ON tasks(workspace_id, status);
    CREATE TABLE task_events (
        task_id INTEGER NOT NULL REFERENCES tasks(id),
        seq INTEGER NOT NULL,
        at INTEGER NOT NULL,
        from_status TEXT,
        to_status TEXT NOT NULL,
        actor TEXT NOT NULL,
        note TEXT NOT NULL DEFAULT '',
        PRIMARY KEY (task_id, seq)
    );
",
    "
    ALTER TABLE workspaces ADD COLUMN isolation TEXT NOT NULL DEFAULT 'none';
    CREATE TABLE runs (
        id INTEGER PRIMARY KEY,
        workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
        task_id INTEGER NOT NULL REFERENCES tasks(id),
        title TEXT NOT NULL,
        action TEXT NOT NULL,
        role TEXT NOT NULL,
        runtime TEXT NOT NULL,
        state TEXT NOT NULL,
        base_sha TEXT,
        branch TEXT,
        worktree_path TEXT,
        started_at INTEGER NOT NULL,
        finished_at INTEGER,
        summary TEXT NOT NULL DEFAULT '',
        blocked TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX runs_workspace ON runs(workspace_id);
    CREATE TABLE run_events (
        run_id INTEGER NOT NULL REFERENCES runs(id),
        seq INTEGER NOT NULL,
        kind TEXT NOT NULL,
        text TEXT NOT NULL,
        PRIMARY KEY (run_id, seq)
    );
",
    "
    ALTER TABLE runs ADD COLUMN model TEXT;
    ALTER TABLE runs ADD COLUMN cycles INTEGER;
    ALTER TABLE runs ADD COLUMN turns INTEGER;
    ALTER TABLE runs ADD COLUMN input_tokens INTEGER;
    ALTER TABLE runs ADD COLUMN output_tokens INTEGER;
    ALTER TABLE runs ADD COLUMN cache_read_tokens INTEGER;
    ALTER TABLE runs ADD COLUMN cache_write_tokens INTEGER;
    ALTER TABLE runs ADD COLUMN cost_usd REAL;
    ALTER TABLE runs ADD COLUMN agent_ms INTEGER;
    ALTER TABLE runs ADD COLUMN checks_ms INTEGER;
    CREATE TABLE run_cycles (
        run_id INTEGER NOT NULL REFERENCES runs(id),
        cycle INTEGER NOT NULL,
        model TEXT NOT NULL,
        session_id TEXT,
        input_tokens INTEGER,
        output_tokens INTEGER,
        cache_read_tokens INTEGER,
        cache_write_tokens INTEGER,
        cost_usd REAL,
        turns INTEGER,
        \"trigger\" TEXT NOT NULL CHECK (\"trigger\" IN ('initial', 'check_failed')),
        started_at INTEGER NOT NULL,
        finished_at INTEGER NOT NULL,
        PRIMARY KEY (run_id, cycle)
    );
",
    "
    ALTER TABLE runs ADD COLUMN proof TEXT CHECK (proof IS NULL OR proof IN ('proved', 'not_proved', 'no_tests', 'skipped', 'inconclusive'));
",
    "
    ALTER TABLE runs ADD COLUMN effort TEXT;
    CREATE TABLE run_cycles_next (
        run_id INTEGER NOT NULL REFERENCES runs(id),
        cycle INTEGER NOT NULL,
        model TEXT NOT NULL,
        session_id TEXT,
        resumed INTEGER NOT NULL DEFAULT 0,
        input_tokens INTEGER,
        output_tokens INTEGER,
        cache_read_tokens INTEGER,
        cache_write_tokens INTEGER,
        cost_usd REAL,
        turns INTEGER,
        \"trigger\" TEXT NOT NULL CHECK (\"trigger\" IN ('initial', 'check_failed', 'diff_guard', 'proof', 'review', 'bigger_model')),
        started_at INTEGER NOT NULL,
        finished_at INTEGER NOT NULL,
        PRIMARY KEY (run_id, cycle)
    );
    INSERT INTO run_cycles_next(run_id, cycle, model, session_id, input_tokens, output_tokens,
        cache_read_tokens, cache_write_tokens, cost_usd, turns, \"trigger\", started_at, finished_at)
        SELECT run_id, cycle, model, session_id, input_tokens, output_tokens,
        cache_read_tokens, cache_write_tokens, cost_usd, turns, \"trigger\", started_at, finished_at
        FROM run_cycles;
    DROP TABLE run_cycles;
    ALTER TABLE run_cycles_next RENAME TO run_cycles;
    CREATE TABLE workspace_denials (
        workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
        command TEXT NOT NULL,
        count INTEGER NOT NULL DEFAULT 1,
        last_seen INTEGER NOT NULL,
        PRIMARY KEY (workspace_id, command)
    );
",
    "
    ALTER TABLE runs ADD COLUMN review TEXT CHECK (review IS NULL OR review IN ('approve', 'request_changes', 'invalid'));
    ALTER TABLE runs ADD COLUMN review_discarded INTEGER;
",
    "
    ALTER TABLE tasks ADD COLUMN parent_id INTEGER REFERENCES tasks(id);
    ALTER TABLE tasks ADD COLUMN after_id INTEGER REFERENCES tasks(id);
    ALTER TABLE runs ADD COLUMN base_run_id INTEGER REFERENCES runs(id);
",
    "
    CREATE TABLE action_approvals (
        workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
        arquivo TEXT NOT NULL,
        sha256 TEXT NOT NULL,
        approved_at INTEGER NOT NULL,
        PRIMARY KEY (workspace_id, arquivo)
    );
",
];

/// Segundos desde a época Unix; `0` se o relógio estiver antes dela.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

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
