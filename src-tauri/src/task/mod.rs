//! Tarefas locais de um workspace: conteúdo, estado e histórico de transições.
//!
//! Cada edição exige a revisão lida pelo cliente; divergência indica que a
//! tarefa mudou e a interface preserva o texto do usuário em vez de sobrescrever.
//! O estado (`todo → doing → review → done`) muda por [`transition`], que grava
//! o novo estado e o evento de histórico na mesma transação. Qualquer estado
//! pode ir para qualquer outro: o histórico é a trava, não um grafo rígido.

pub mod commands;

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::{
    db::now,
    error::{AppError, Result},
};

const MAX_TITLE_BYTES: usize = 300;
const MAX_TEXT_BYTES: usize = 100_000;
const MAX_NOTE_BYTES: usize = 280;
const HISTORY_LIMIT: i64 = 50;
const COLUMNS: &str =
    "id, workspace_id, title, description, criteria, revision, status, parent_id, after_id";
/// Passos criados de uma vez a partir de "Dividir em passos".
const MAX_STEPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Todo,
    Doing,
    Review,
    Done,
}

impl Status {
    const ALL: [Status; 4] = [Self::Todo, Self::Doing, Self::Review, Self::Done];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Doing => "doing",
            Self::Review => "review",
            Self::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
            .ok_or(AppError::Validation(
                "Estado inválido. Use todo, doing, review ou done.",
            ))
    }
}

/// Quem causou a transição, gravado no histórico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    User,
    System,
}

impl Actor {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub workspace_id: i64,
    pub title: String,
    pub description: String,
    pub criteria: String,
    pub revision: i64,
    pub status: Status,
    /// Tarefa de origem quando esta é um passo de "Dividir em passos".
    pub parent_id: Option<i64>,
    /// Passo anterior da cadeia; a run deste passo pode partir da branch dele.
    pub after_id: Option<i64>,
}

impl Task {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        let status: String = r.get(6)?;
        Ok(Self {
            id: r.get(0)?,
            workspace_id: r.get(1)?,
            title: r.get(2)?,
            description: r.get(3)?,
            criteria: r.get(4)?,
            revision: r.get(5)?,
            status: Status::parse(&status).map_err(|_| rusqlite::Error::InvalidQuery)?,
            parent_id: r.get(7)?,
            after_id: r.get(8)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub seq: i64,
    pub at: i64,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor: String,
    pub note: String,
}

/// Resultado de uma transição, enviado à interface em `task-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub task_id: i64,
    pub status: Status,
    pub revision: i64,
    pub event: TaskEvent,
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

/// Um passo confirmado pelo usuário: vira uma Tarefa normal.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub title: String,
    pub criteria: String,
}

/// Cria os passos como Tarefas em `todo`, em ordem, cada um apontando para a
/// origem (`parent_id`) e para o passo anterior (`after_id`). Tudo ou nada.
pub fn create_steps(
    db: &Connection,
    workspace_id: i64,
    parent_id: i64,
    steps: &[Step],
) -> Result<Vec<Task>> {
    let parent = get(db, workspace_id, parent_id)?;
    if steps.is_empty() || steps.len() > MAX_STEPS {
        return Err(AppError::Validation("Confirme de 1 a 5 passos."));
    }
    let tx = db.unchecked_transaction()?;
    let mut previous: Option<i64> = None;
    let mut ids = Vec::new();
    for step in steps {
        let input = TaskInput {
            workspace_id,
            id: None,
            revision: None,
            title: step.title.clone(),
            description: format!("Passo de \"{}\".", parent.title),
            criteria: step.criteria.clone(),
        };
        let title = input.validated_title()?;
        tx.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria, parent_id, after_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                workspace_id,
                title,
                input.description,
                input.criteria,
                parent.id,
                previous
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO task_events(task_id, seq, at, from_status, to_status, actor)
             VALUES (?1, 1, ?2, NULL, 'todo', ?3)",
            params![id, now(), Actor::User.as_str()],
        )?;
        previous = Some(id);
        ids.push(id);
    }
    tx.commit()?;
    ids.into_iter()
        .map(|id| get(db, workspace_id, id))
        .collect()
}

pub fn list(db: &Connection, workspace_id: i64) -> Result<Vec<Task>> {
    let mut query = db.prepare(&format!(
        "SELECT {COLUMNS} FROM tasks WHERE workspace_id=?1 ORDER BY id DESC"
    ))?;
    let rows = query.query_map([workspace_id], Task::from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get(db: &Connection, workspace_id: i64, id: i64) -> Result<Task> {
    db.query_row(
        &format!("SELECT {COLUMNS} FROM tasks WHERE id=?1 AND workspace_id=?2"),
        [id, workspace_id],
        Task::from_row,
    )
    .map_err(|_| AppError::Validation("Tarefa não encontrada."))
}

/// Remove a tarefa e o histórico dela. Passos que dependiam dela perdem só o vínculo.
/// As runs são apagadas por quem orquestra ([`crate::flow`]), que também abre a
/// transação: esta função não abre uma própria.
pub fn delete(db: &Connection, workspace_id: i64, id: i64) -> Result<()> {
    get(db, workspace_id, id)?;
    db.execute("UPDATE tasks SET parent_id=NULL WHERE parent_id=?1", [id])?;
    db.execute("UPDATE tasks SET after_id=NULL WHERE after_id=?1", [id])?;
    db.execute("DELETE FROM task_events WHERE task_id=?1", [id])?;
    db.execute("DELETE FROM tasks WHERE id=?1", [id])?;
    Ok(())
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
    let tx = db.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (?1, ?2, ?3, ?4)",
        params![input.workspace_id, title, input.description, input.criteria],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO task_events(task_id, seq, at, from_status, to_status, actor)
         VALUES (?1, 1, ?2, NULL, 'todo', ?3)",
        params![id, now(), Actor::User.as_str()],
    )?;
    tx.commit()?;
    Ok(id)
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

/// Move a tarefa para `to` e registra o evento. Sem efeito (`None`) quando o
/// estado já é `to` e não há nota; com nota, vira uma anotação no histórico.
/// A última escrita vence e `revision` não muda: ela só protege edições de
/// conteúdo, e mover o card não pode invalidar um editor aberto.
pub fn transition(
    db: &Connection,
    task_id: i64,
    to: Status,
    actor: Actor,
    note: &str,
) -> Result<Option<Change>> {
    let note = clip(note.trim(), MAX_NOTE_BYTES);
    let tx = db.unchecked_transaction()?;
    let current: String = tx
        .query_row("SELECT status FROM tasks WHERE id=?1", [task_id], |r| {
            r.get(0)
        })
        .map_err(|_| AppError::Validation("Tarefa não encontrada."))?;
    if current == to.as_str() && note.is_empty() {
        return Ok(None);
    }
    let moved = current != to.as_str();
    if moved {
        tx.execute(
            "UPDATE tasks SET status=?2 WHERE id=?1",
            params![task_id, to.as_str()],
        )?;
    }
    let seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM task_events WHERE task_id=?1",
        [task_id],
        |r| r.get(0),
    )?;
    let event = TaskEvent {
        seq,
        at: now(),
        from_status: Some(current),
        to_status: to.as_str().to_owned(),
        actor: actor.as_str().to_owned(),
        note,
    };
    tx.execute(
        "INSERT INTO task_events(task_id, seq, at, from_status, to_status, actor, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            task_id,
            event.seq,
            event.at,
            event.from_status,
            event.to_status,
            event.actor,
            event.note
        ],
    )?;
    let revision: i64 = tx.query_row("SELECT revision FROM tasks WHERE id=?1", [task_id], |r| {
        r.get(0)
    })?;
    tx.commit()?;
    Ok(Some(Change {
        task_id,
        status: to,
        revision,
        event,
    }))
}

/// Últimos eventos da tarefa, do mais recente para o mais antigo.
pub fn history(db: &Connection, task_id: i64) -> Result<Vec<TaskEvent>> {
    let mut query = db.prepare(
        "SELECT seq, at, from_status, to_status, actor, note FROM task_events
         WHERE task_id=?1 ORDER BY seq DESC LIMIT ?2",
    )?;
    let rows = query.query_map(params![task_id, HISTORY_LIMIT], |r| {
        Ok(TaskEvent {
            seq: r.get(0)?,
            at: r.get(1)?,
            from_status: r.get(2)?,
            to_status: r.get(3)?,
            actor: r.get(4)?,
            note: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
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

    fn workspace(db: &Connection) -> i64 {
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('Piloto', '/tmp/piloto')",
            [],
        )
        .unwrap();
        db.last_insert_rowid()
    }

    #[test]
    fn steps_become_chained_tasks_all_or_nothing() {
        let temp = TempDatabase::new("task-steps");
        let store = temp.open();
        let db = store.connection().unwrap();
        let ws = workspace(&db);
        let parent = save(&db, &input(ws, None, None, "Grande")).unwrap();
        let step = |title: &str| Step {
            title: title.into(),
            criteria: "- feito".into(),
        };
        let created = create_steps(
            &db,
            ws,
            parent.id,
            &[step("Um"), step("Dois"), step("Três")],
        )
        .unwrap();
        assert_eq!(created.len(), 3);
        assert!(created
            .iter()
            .all(|t| t.parent_id == Some(parent.id) && t.status == Status::Todo));
        assert_eq!(created[0].after_id, None);
        assert_eq!(created[1].after_id, Some(created[0].id));
        assert_eq!(created[2].after_id, Some(created[1].id));
        assert_eq!(history(&db, created[2].id).unwrap().len(), 1);

        let before = list(&db, ws).unwrap().len();
        assert!(create_steps(&db, ws, parent.id, &[step("ok"), step("  ")]).is_err());
        assert!(create_steps(&db, ws, parent.id, &[]).is_err());
        let six: Vec<Step> = (0..6).map(|i| step(&format!("p{i}"))).collect();
        assert!(create_steps(&db, ws, parent.id, &six).is_err());
        assert!(create_steps(&db, ws + 1, parent.id, &[step("x")]).is_err());
        assert_eq!(
            list(&db, ws).unwrap().len(),
            before,
            "falha não deixa passo pela metade"
        );
    }

    #[test]
    fn persists_tasks_and_rejects_stale_edits() {
        let temp = TempDatabase::new("task");
        let workspace_id = {
            let store = temp.open();
            let db = store.connection().unwrap();
            let workspace_id = workspace(&db);

            let task = save(&db, &input(workspace_id, None, None, "Primeira tarefa")).unwrap();
            assert_eq!(task.status, Status::Todo);
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

    #[test]
    fn transitions_record_history_and_skip_noops() {
        let temp = TempDatabase::new("task-transition");
        let store = temp.open();
        let db = store.connection().unwrap();
        let ws = workspace(&db);
        let task = save(&db, &input(ws, None, None, "Mover")).unwrap();

        assert!(transition(&db, task.id, Status::Todo, Actor::User, "")
            .unwrap()
            .is_none());
        let change = transition(&db, task.id, Status::Review, Actor::System, "pronto")
            .unwrap()
            .unwrap();
        assert_eq!(change.status, Status::Review);
        assert_eq!(change.revision, 1, "mover não invalida o editor aberto");
        assert_eq!(change.event.from_status.as_deref(), Some("todo"));
        assert_eq!(get(&db, ws, task.id).unwrap().status, Status::Review);

        let note = transition(&db, task.id, Status::Review, Actor::User, &"x".repeat(400))
            .unwrap()
            .unwrap();
        assert_eq!(note.revision, 1, "anotação não muda a revisão");
        assert_eq!(note.event.note.len(), MAX_NOTE_BYTES);

        let events = history(&db, task.id).unwrap();
        let seqs: Vec<i64> = events.iter().map(|e| e.seq).collect();
        assert_eq!(seqs, [3, 2, 1]);
        assert_eq!(events[1].actor, "system");
        assert_eq!(events[2].from_status, None);
        assert!(transition(&db, 999, Status::Done, Actor::User, "").is_err());
    }

    #[test]
    fn status_parses_exact_names_only() {
        assert_eq!(Status::parse("review").unwrap(), Status::Review);
        assert!(Status::parse("rev").is_err());
        assert!(Status::parse(" done").is_err());
        assert!(Status::parse("").is_err());
        assert!(Status::parse("pronto").is_err());
    }

    #[test]
    fn check_constraint_rejects_unknown_status() {
        let temp = TempDatabase::new("task-check");
        let store = temp.open();
        let db = store.connection().unwrap();
        let ws = workspace(&db);
        let task = save(&db, &input(ws, None, None, "T")).unwrap();
        assert!(db
            .execute("UPDATE tasks SET status='x' WHERE id=?1", [task.id])
            .is_err());
        let version: usize = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert!(version >= 4, "migrações aplicadas: {version}");
    }
}
