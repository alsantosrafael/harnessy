//! Tarefas locais de um workspace, com controle otimista de revisão.
//!
//! Cada edição exige a revisão lida pelo cliente; divergência indica que a
//! tarefa mudou e a interface preserva o texto do usuário em vez de sobrescrever.

pub mod commands;

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

const MAX_TITLE_BYTES: usize = 300;
const MAX_TEXT_BYTES: usize = 100_000;
const COLUMNS: &str = "id, workspace_id, title, description, criteria, revision";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub workspace_id: i64,
    pub title: String,
    pub description: String,
    pub criteria: String,
    pub revision: i64,
}

impl Task {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            workspace_id: r.get(1)?,
            title: r.get(2)?,
            description: r.get(3)?,
            criteria: r.get(4)?,
            revision: r.get(5)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskInput {
    pub workspace_id: i64,
    /// `None` cria uma tarefa; `Some` atualiza a existente.
    pub id: Option<i64>,
    /// Revisão vista pelo cliente ao abrir a tarefa; obrigatória em atualizações.
    pub revision: Option<i64>,
    pub title: String,
    pub description: String,
    pub criteria: String,
}

impl TaskInput {
    fn validated_title(&self) -> Result<&str> {
        let title = self.title.trim();
        if title.is_empty()
            || title.len() > MAX_TITLE_BYTES
            || self.description.len() > MAX_TEXT_BYTES
            || self.criteria.len() > MAX_TEXT_BYTES
        {
            return Err(AppError::Validation(
                "Informe um título (até 300 bytes) e textos de até 100 KB por campo.",
            ));
        }
        Ok(title)
    }
}

pub fn list(db: &Connection, workspace_id: i64) -> Result<Vec<Task>> {
    let mut query = db.prepare(&format!(
        "SELECT {COLUMNS} FROM tasks WHERE workspace_id=?1 ORDER BY id DESC"
    ))?;
    let rows = query.query_map([workspace_id], Task::from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn save(db: &Connection, input: &TaskInput) -> Result<Task> {
    let title = input.validated_title()?;
    let id = match input.id {
        Some(id) => update(db, id, title, input)?,
        None => insert(db, title, input)?,
    };
    find(db, id)
}

fn insert(db: &Connection, title: &str, input: &TaskInput) -> Result<i64> {
    db.execute(
        "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (?1, ?2, ?3, ?4)",
        params![input.workspace_id, title, input.description, input.criteria],
    )?;
    Ok(db.last_insert_rowid())
}

fn update(db: &Connection, id: i64, title: &str, input: &TaskInput) -> Result<i64> {
    let changed = db.execute(
        "UPDATE tasks SET title=?1, description=?2, criteria=?3, revision=revision+1
         WHERE id=?4 AND workspace_id=?5 AND revision=?6",
        params![
            title,
            input.description,
            input.criteria,
            id,
            input.workspace_id,
            input.revision
        ],
    )?;
    if changed != 1 {
        return Err(AppError::StaleTask);
    }
    Ok(id)
}

fn find(db: &Connection, id: i64) -> Result<Task> {
    Ok(db.query_row(
        &format!("SELECT {COLUMNS} FROM tasks WHERE id=?1"),
        [id],
        Task::from_row,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    fn input(workspace_id: i64, id: Option<i64>, revision: Option<i64>, title: &str) -> TaskInput {
        TaskInput {
            workspace_id,
            id,
            revision,
            title: title.into(),
            description: "Descrição real".into(),
            criteria: "Reabrir sem perder conteúdo".into(),
        }
    }

    #[test]
    fn persists_tasks_and_rejects_stale_edits() {
        let temp = TempDatabase::new("task");
        let workspace_id = {
            let store = temp.open();
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('Piloto', '/tmp/piloto')",
                [],
            )
            .unwrap();
            let workspace_id = db.last_insert_rowid();

            let task = save(&db, &input(workspace_id, None, None, "Primeira tarefa")).unwrap();
            let updated = save(
                &db,
                &input(workspace_id, Some(task.id), Some(1), "Tarefa editada"),
            )
            .unwrap();
            assert_eq!(updated.revision, 2);
            assert!(matches!(
                save(
                    &db,
                    &input(workspace_id, Some(task.id), Some(1), "Edição antiga")
                ),
                Err(AppError::StaleTask)
            ));
            assert!(matches!(
                save(&db, &input(workspace_id, None, None, "  ")),
                Err(AppError::Validation(_))
            ));
            workspace_id
        };

        let reopened = temp.open();
        let db = reopened.connection().unwrap();
        let restored = list(&db, workspace_id).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].title, "Tarefa editada");
        assert_eq!(restored[0].description, "Descrição real");
        assert_eq!(restored[0].criteria, "Reabrir sem perder conteúdo");
    }
}
