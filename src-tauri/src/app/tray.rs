//! Ícone de bandeja e eventos de janela. Fechar oculta; a bandeja reabre ou encerra.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, RunEvent, Window, WindowEvent,
};

use super::show_main_window;

const OPEN: &str = "open";
const QUIT: &str = "quit";

pub fn install(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN, "Abrir Harnessy", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Encerrar Harnessy", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::new()
        .title("H")
        .tooltip("Harnessy — aplicativo local")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            OPEN => show_main_window(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        if let Err(error) = window.hide() {
            eprintln!("[harnessy] Falha ao ocultar janela: {error}");
        }
    }
}

pub fn on_run_event(app: &AppHandle, event: RunEvent) {
    if let RunEvent::Reopen { .. } = event {
        show_main_window(app);
    }
}
