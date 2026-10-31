//! Ícone de bandeja e eventos de janela. Fechar oculta; a bandeja reabre ou encerra.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager, RunEvent, Window, WindowEvent,
};

use super::show_main_window;

/// Identificador do ícone, para o contador de atenção encontrá-lo depois.
pub const TRAY_ID: &str = "main";
const OPEN: &str = "open";
const QUIT: &str = "quit";

pub fn install(app: &App) -> tauri::Result<()> {
    let tag = super::instance_tag(&app.config().identifier);
    let open = MenuItem::with_id(app, OPEN, "Abrir Harnessy", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Encerrar Harnessy", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .title(super::attention::tray_title(0, tag.as_deref()))
        .tooltip(match tag.as_deref() {
            Some(tag) => format!("Harnessy ({tag}) — instância de teste, com dados separados"),
            None => "Harnessy — aplicativo local".to_owned(),
        })
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
    match event {
        RunEvent::Reopen { .. } => show_main_window(app),
        // Não deixa servidor de desenvolvimento órfão ocupando a porta depois de fechar o app.
        RunEvent::Exit => app.state::<crate::run::preview::Previews>().stop_all(),
        _ => {}
    }
}
