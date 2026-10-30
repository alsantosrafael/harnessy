//! Ciclo de vida do aplicativo: metadados, estado gerenciado e janela.

pub mod tray;

use serde::Serialize;
use tauri::{App, AppHandle, Manager};

use crate::db::Database;

const DATABASE_FILE: &str = "local-work.sqlite3";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    platform: &'static str,
    agent_execution_available: bool,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let version = app.package_info().version.to_string();
    eprintln!("[harnessy] IPC app_info: version={version}");
    AppInfo {
        version,
        platform: std::env::consts::OS,
        agent_execution_available: false,
    }
}

/// Prepara o estado gerenciado e a bandeja. Chamado uma vez pelo `Builder`.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    app.manage(Database::open(&data_dir.join(DATABASE_FILE))?);
    tray::install(app)?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show().and_then(|_| window.set_focus()) {
            eprintln!("[harnessy] Falha ao reabrir janela: {error}");
        }
    }
}
