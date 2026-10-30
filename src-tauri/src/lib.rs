//! Núcleo nativo do Harnessy.
//!
//! Este arquivo apenas conecta os módulos ao `tauri::Builder`. Regras de
//! negócio ficam nos módulos de domínio; adaptadores IPC em `*/commands.rs`.
//! Veja `docs/architecture-rust.md` para o mapa completo e as regras.

mod action;
mod app;
mod connections;
mod db;
mod error;
mod flow;
mod process;
mod run;
mod task;
mod workspace;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app::app_info,
            workspace::commands::list_workspaces,
            workspace::commands::create_workspace,
            task::commands::list_tasks,
            task::commands::save_task,
            task::commands::move_task,
            task::commands::task_history,
            task::commands::create_steps,
            connections::check_connections,
            flow::commands::preview_prompt,
            flow::commands::start_run,
            flow::commands::continue_run,
            flow::commands::approve_action,
            flow::commands::create_action,
            flow::commands::resolve_block,
            flow::commands::list_actions,
            run::commands::list_runs,
            run::commands::list_run_events,
            run::commands::cancel_run,
        ])
        .setup(app::setup)
        .on_window_event(app::tray::on_window_event)
        .build(tauri::generate_context!())
        .expect("Não foi possível inicializar o Harnessy")
        .run(app::tray::on_run_event);
}
