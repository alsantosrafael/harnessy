//! Adaptadores IPC: obtêm a conexão e delegam ao domínio.

use tauri::State;

use super::{Task, TaskInput};
use crate::{db::Database, error::Result};

#[tauri::command]
pub fn list_tasks(store: State<'_, Database>, workspace_id: i64) -> Result<Vec<Task>> {
    super::list(&*store.connection()?, workspace_id)
}

#[tauri::command]
pub fn save_task(store: State<'_, Database>, input: TaskInput) -> Result<Task> {
    super::save(&*store.connection()?, &input)
}
