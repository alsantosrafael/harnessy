use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};

pub struct LocalWork(Mutex<Connection>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    id: i64,
    name: String,
    folder: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    id: i64,
    workspace_id: i64,
    title: String,
    description: String,
    criteria: String,
    revision: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskInput {
    workspace_id: i64,
    id: Option<i64>,
    revision: Option<i64>,
    title: String,
    description: String,
    criteria: String,
}

fn db_error(_: rusqlite::Error) -> String {
    "Não foi possível acessar o banco local. Verifique espaço e permissões e tente novamente."
        .into()
}

impl LocalWork {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db = Connection::open(path).map_err(db_error)?;
        db.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(db_error)?;
        db.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(db_error)?;
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_error)?;
        if version > 1 {
            return Err(
                "Banco criado por uma versão mais recente do Harnessy. Atualize o aplicativo."
                    .into(),
            );
        }
        if version == 0 {
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE workspaces (id INTEGER PRIMARY KEY, name TEXT NOT NULL, folder TEXT NOT NULL UNIQUE);
                CREATE TABLE tasks (id INTEGER PRIMARY KEY, workspace_id INTEGER NOT NULL REFERENCES workspaces(id), title TEXT NOT NULL, description TEXT NOT NULL, criteria TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1);
                CREATE INDEX tasks_workspace ON tasks(workspace_id);
                PRAGMA user_version=1;
                COMMIT;").map_err(db_error)?;
        }
        Ok(Self(Mutex::new(db)))
    }

    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.0
            .lock()
            .map_err(|_| "Banco local indisponível. Reinicie o aplicativo.".into())
    }
}

#[tauri::command]
pub fn list_workspaces(store: tauri::State<'_, LocalWork>) -> Result<Vec<Workspace>, String> {
    let db = store.connection()?;
    let mut query = db
        .prepare("SELECT id, name, folder FROM workspaces ORDER BY id")
        .map_err(db_error)?;
    let rows = query
        .query_map([], |r| {
            Ok(Workspace {
                id: r.get(0)?,
                name: r.get(1)?,
                folder: r.get(2)?,
            })
        })
        .map_err(db_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}

#[tauri::command]
pub fn create_workspace(
    store: tauri::State<'_, LocalWork>,
    name: String,
    folder: String,
) -> Result<Workspace, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err("Informe um nome de até 200 bytes.".into());
    }
    let path = Path::new(folder.trim());
    if !path.is_absolute() {
        return Err("Informe o caminho absoluto de uma pasta local.".into());
    }
    let path = path
        .canonicalize()
        .map_err(|_| "Pasta não encontrada ou sem permissão de acesso.".to_string())?;
    if !path.is_dir() {
        return Err("O caminho deve apontar para uma pasta.".into());
    }
    let folder = path
        .to_str()
        .ok_or("O caminho precisa conter texto UTF-8 válido.")?
        .to_string();
    let db = store.connection()?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE folder=?1)",
            [&folder],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    if exists {
        return Err("Esta pasta já está vinculada a um workspace.".into());
    }
    db.execute(
        "INSERT INTO workspaces(name, folder) VALUES (?1, ?2)",
        params![name, folder],
    )
    .map_err(db_error)?;
    Ok(Workspace {
        id: db.last_insert_rowid(),
        name: name.into(),
        folder,
    })
}

fn task_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        workspace_id: r.get(1)?,
        title: r.get(2)?,
        description: r.get(3)?,
        criteria: r.get(4)?,
        revision: r.get(5)?,
    })
}

#[tauri::command]
pub fn list_tasks(
    store: tauri::State<'_, LocalWork>,
    workspace_id: i64,
) -> Result<Vec<Task>, String> {
    let db = store.connection()?;
    let mut query = db.prepare("SELECT id, workspace_id, title, description, criteria, revision FROM tasks WHERE workspace_id=?1 ORDER BY id DESC").map_err(db_error)?;
    let rows = query
        .query_map([workspace_id], task_row)
        .map_err(db_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}

fn save(db: &Connection, input: TaskInput) -> Result<Task, String> {
    let title = input.title.trim();
    if title.is_empty()
        || title.len() > 300
        || input.description.len() > 100_000
        || input.criteria.len() > 100_000
    {
        return Err("Informe um título (até 300 bytes) e textos de até 100 KB por campo.".into());
    }
    let id = if let Some(id) = input.id {
        let changed = db.execute("UPDATE tasks SET title=?1, description=?2, criteria=?3, revision=revision+1 WHERE id=?4 AND workspace_id=?5 AND revision=?6", params![title, input.description, input.criteria, id, input.workspace_id, input.revision]).map_err(db_error)?;
        if changed != 1 {
            return Err("A tarefa foi alterada ou não existe mais. Seu texto foi preservado; reabra a tarefa antes de aplicar as alterações.".into());
        }
        id
    } else {
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (?1, ?2, ?3, ?4)",
            params![input.workspace_id, title, input.description, input.criteria],
        )
        .map_err(db_error)?;
        db.last_insert_rowid()
    };
    db.query_row(
        "SELECT id, workspace_id, title, description, criteria, revision FROM tasks WHERE id=?1",
        [id],
        task_row,
    )
    .map_err(db_error)
}

#[tauri::command]
pub fn save_task(store: tauri::State<'_, LocalWork>, input: TaskInput) -> Result<Task, String> {
    let db = store.connection()?;
    save(&db, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_tasks_and_rejects_stale_edits() {
        let path =
            std::env::temp_dir().join(format!("harnessy-test-{}.sqlite", std::process::id()));
        let store = LocalWork::open(&path).unwrap();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('Piloto', '/tmp/piloto')",
            [],
        )
        .unwrap();
        let workspace_id = db.last_insert_rowid();
        let input = |id, revision, title: &str| TaskInput {
            workspace_id,
            id,
            revision,
            title: title.into(),
            description: "Descrição real".into(),
            criteria: "Reabrir sem perder conteúdo".into(),
        };
        let task = save(&db, input(None, None, "Primeira tarefa")).unwrap();
        let updated = save(&db, input(Some(task.id), Some(1), "Tarefa editada")).unwrap();
        assert_eq!(updated.revision, 2);
        assert!(save(&db, input(Some(task.id), Some(1), "Edição antiga")).is_err());
        assert!(save(&db, input(None, None, "  ")).is_err());
        drop(db);
        drop(store);
        let reopened = LocalWork::open(&path).unwrap();
        let db = reopened.connection().unwrap();
        let restored = db
            .query_row(
                "SELECT id, workspace_id, title, description, criteria, revision FROM tasks",
                [],
                task_row,
            )
            .unwrap();
        assert_eq!(restored.title, "Tarefa editada");
        assert_eq!(restored.description, "Descrição real");
        assert_eq!(restored.criteria, "Reabrir sem perder conteúdo");
        drop(db);
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
}
