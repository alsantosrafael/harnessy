//! Adaptadores IPC: obtêm a conexão e delegam ao domínio.

use tauri::{AppHandle, Emitter, State};

use super::{Actor, Change, Status, Step, Task, TaskEvent, TaskInput};
use crate::{db::Database, error::Result};

/// Avisa a interface de uma transição; o payload é só o necessário para aplicar o patch.
pub fn emit_change(app: &AppHandle, change: &Change) {
    let _ = app.emit("task-changed", change);
}

#[tauri::command]
pub async fn list_tasks(store: State<'_, Database>, workspace_id: i64) -> Result<Vec<Task>> {
    super::list(&*store.connection()?, workspace_id)
}

#[tauri::command]
pub async fn save_task(store: State<'_, Database>, input: TaskInput) -> Result<Task> {
    super::save(&*store.connection()?, &input)
}

#[tauri::command]
pub async fn move_task(
    app: AppHandle,
    store: State<'_, Database>,
    task_id: i64,
    to: Status,
) -> Result<()> {
    let change = super::transition(&*store.connection()?, task_id, to, Actor::User, "")?;
    if let Some(change) = &change {
        emit_change(&app, change);
    }
    Ok(())
}

#[tauri::command]
pub async fn task_history(store: State<'_, Database>, task_id: i64) -> Result<Vec<TaskEvent>> {
    super::history(&*store.connection()?, task_id)
}

/// Cria os passos confirmados de "Dividir em passos" como Tarefas encadeadas.
#[tauri::command]
pub async fn create_steps(
    store: State<'_, Database>,
    workspace_id: i64,
    parent_id: i64,
    steps: Vec<Step>,
) -> Result<Vec<Task>> {
    super::create_steps(&*store.connection()?, workspace_id, parent_id, &steps)
}
