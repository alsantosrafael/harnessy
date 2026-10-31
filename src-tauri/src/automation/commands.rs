//! Adaptadores IPC da automação. A decisão e a execução das etapas ficam em `flow`.

use tauri::State;

use super::{Event, Guardrails, Modes, Overrides, Settings, View};
use crate::{db::Database, error::Result};

#[tauri::command]
pub async fn get_automation(
    store: State<'_, Database>,
    workspace_id: i64,
    task_id: Option<i64>,
) -> Result<View> {
    super::view(&*store.connection()?, workspace_id, task_id)
}

#[tauri::command]
pub async fn save_automation(
    store: State<'_, Database>,
    workspace_id: i64,
    modes: Modes,
    guardrails: Guardrails,
) -> Result<Settings> {
    super::save_settings(&*store.connection()?, workspace_id, &modes, &guardrails)
}

#[tauri::command]
pub async fn apply_automation_preset(
    store: State<'_, Database>,
    workspace_id: i64,
    preset: String,
) -> Result<Settings> {
    super::apply_preset(&*store.connection()?, workspace_id, &preset)
}

#[tauri::command]
pub async fn set_automation_paused(
    store: State<'_, Database>,
    workspace_id: i64,
    paused: bool,
) -> Result<Settings> {
    super::set_paused(&*store.connection()?, workspace_id, paused)
}

#[tauri::command]
pub async fn save_task_automation(
    store: State<'_, Database>,
    task_id: i64,
    overrides: Overrides,
) -> Result<Overrides> {
    super::save_overrides(&*store.connection()?, task_id, &overrides)
}

#[tauri::command]
pub async fn list_automation_events(
    store: State<'_, Database>,
    workspace_id: i64,
    task_id: Option<i64>,
    limit: u32,
) -> Result<Vec<Event>> {
    super::events(&*store.connection()?, workspace_id, task_id, limit)
}
