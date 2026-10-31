//! Ciclo de vida do aplicativo: metadados, estado gerenciado e janela.

pub mod attention;
pub mod tray;

use serde::Serialize;
use tauri::{App, AppHandle, Manager};

use crate::{db::Database, flow, process, run::supervisor::Registry};

const DATABASE_FILE: &str = "local-work.sqlite3";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    platform: &'static str,
    agent_execution_available: bool,
    /// Etiqueta da instância de teste (`dogfood`); `None` na principal.
    instance: Option<String>,
}

/// Identificador da instância principal (a que o usuário usa no dia a dia).
const MAIN_IDENTIFIER: &str = "app.harnessy.desktop";
const MAX_TAG_CHARS: usize = 24;

/// Etiqueta de uma instância de teste (`dogfood`, `preview`…): o que vem depois do identificador
/// principal. Cada uma tem dados, ícone e branches próprios; `None` na instância principal.
pub fn instance_tag(identifier: &str) -> Option<String> {
    let tag = identifier
        .strip_prefix(MAIN_IDENTIFIER)?
        .strip_prefix('.')?;
    let valid = !tag.is_empty()
        && tag.chars().count() <= MAX_TAG_CHARS
        && tag
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    valid.then(|| tag.to_owned())
}

#[tauri::command]
pub async fn app_info(app: AppHandle) -> AppInfo {
    let version = app.package_info().version.to_string();
    eprintln!("[harnessy] IPC app_info: version={version}");
    AppInfo {
        version,
        platform: std::env::consts::OS,
        agent_execution_available: agent_tools_present(),
        instance: instance_tag(&app.config().identifier),
    }
}

/// Execução de agentes exige `git` e `claude` instalados.
fn agent_tools_present() -> bool {
    let dirs = process::search_dirs();
    ["git", "claude"]
        .iter()
        .all(|name| process::find_binary(name, &dirs).is_some())
}

/// Prepara o estado gerenciado e a bandeja. Chamado uma vez pelo `Builder`.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    process::warm_up();
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let database = Database::open(&data_dir.join(DATABASE_FILE))?;
    // Runs que sobraram da última sessão não têm mais processo: interrompe e libera os cards.
    let read_only = flow::actions::read_only_ids(&data_dir);
    flow::sync::recover(&*database.connection()?, &read_only)?;
    app.manage(database);
    app.manage(Registry::default());
    app.manage(crate::run::preview::Previews::default());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_suffix_after_the_main_identifier_makes_a_test_instance() {
        assert_eq!(instance_tag("app.harnessy.desktop"), None);
        assert_eq!(instance_tag("app.harnessy.desktop."), None);
        assert_eq!(
            instance_tag("app.harnessy.desktop.dogfood").as_deref(),
            Some("dogfood")
        );
        assert_eq!(
            instance_tag("app.harnessy.desktop.preview-2").as_deref(),
            Some("preview-2")
        );
        assert_eq!(instance_tag("app.harnessy.desktop.Maiusculas"), None);
        assert_eq!(instance_tag("app.harnessy.desktop.a/b"), None);
        assert_eq!(instance_tag("app.harnessy.desktopx"), None);
        assert_eq!(instance_tag("com.outra.coisa"), None);
        assert_eq!(
            instance_tag(&format!("app.harnessy.desktop.{}", "a".repeat(40))),
            None
        );
    }
}
