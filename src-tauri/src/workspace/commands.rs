//! Adaptadores IPC: obtêm a conexão e delegam ao domínio.

use tauri::State;

use super::Workspace;
use crate::{db::Database, error::Result};

#[tauri::command]
pub fn list_workspaces(store: State<'_, Database>) -> Result<Vec<Workspace>> {
    super::list(&*store.connection()?)
}

#[tauri::command]
pub fn create_workspace(
    store: State<'_, Database>,
    name: String,
    folder: String,
) -> Result<Workspace> {
    super::create(&*store.connection()?, &name, &folder)
}
