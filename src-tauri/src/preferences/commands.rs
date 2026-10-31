//! Adaptadores IPC das preferências.

use tauri::State;

use crate::{db::Database, error::Result};

#[tauri::command]
pub async fn get_preference(store: State<'_, Database>, key: String) -> Result<Option<String>> {
    super::get(&*store.connection()?, &key)
}

#[tauri::command]
pub async fn set_preference(store: State<'_, Database>, key: String, value: String) -> Result<()> {
    super::set(&*store.connection()?, &key, &value)
}
