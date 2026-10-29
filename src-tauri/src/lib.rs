use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    platform: &'static str,
    agent_execution_available: bool,
}

#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfo {
    eprintln!(
        "[harnessy] IPC app_info: version={}",
        app.package_info().version
    );
    AppInfo {
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        agent_execution_available: false,
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show().and_then(|_| window.set_focus()) {
            eprintln!("[harnessy] Falha ao reabrir janela: {error}");
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Abrir Harnessy", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Encerrar Harnessy", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .title("H")
                .tooltip("Harnessy — aplicativo local")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        show_main_window(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("[harnessy] Falha ao ocultar janela: {error}");
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Não foi possível inicializar o Harnessy")
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app);
            }
        });
}
