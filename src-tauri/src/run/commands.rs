//! Adaptadores IPC de leitura e cancelamento de runs. O início de uma run é
//! orquestrado em `flow`, que cruza tarefas, workspaces e runs.

use tauri::State;

use super::{supervisor::Registry, Run, RunEvent};
use crate::{
    db::Database,
    error::{AppError, Result},
};

#[tauri::command]
pub fn list_runs(store: State<'_, Database>, workspace_id: i64) -> Result<Vec<Run>> {
    super::list(&*store.connection()?, workspace_id)
}

#[tauri::command]
pub fn list_run_events(
    store: State<'_, Database>,
    run_id: i64,
    after_seq: i64,
) -> Result<Vec<RunEvent>> {
    super::events(&*store.connection()?, run_id, after_seq)
}

/// Encerra o processo do agente; o pipeline em `flow` registra o estado final.
#[tauri::command]
pub fn cancel_run(registry: State<'_, Registry>, run_id: i64) -> Result<()> {
    if registry.cancel(run_id) {
        Ok(())
    } else {
        Err(AppError::Validation("Esta run não está em execução."))
    }
}
