//! Núcleo nativo do Harnessy.
//!
//! Este arquivo apenas conecta os módulos ao `tauri::Builder`. Regras de
//! negócio ficam nos módulos de domínio; adaptadores IPC em `*/commands.rs`.
//! Veja `docs/architecture-rust.md` para o mapa completo e as regras.

mod action;
mod app;
mod automation;
mod connections;
mod db;
mod delivery;
mod error;
mod flow;
mod preferences;
mod process;
mod profile;
mod run;
mod task;
mod workspace;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            app::app_info,
            preferences::commands::get_preference,
            preferences::commands::set_preference,
            app::attention::set_attention_count,
            app::attention::notify_attention,
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
            flow::commands::delete_task,
            flow::commands::review_run,
            flow::commands::publish_run,
            flow::commands::refresh_delivery,
            flow::commands::merge_delivery,
            flow::commands::list_deliveries,
            flow::commands::run_diff,
            automation::commands::get_automation,
            automation::commands::save_automation,
            automation::commands::apply_automation_preset,
            automation::commands::set_automation_paused,
            automation::commands::save_task_automation,
            automation::commands::list_automation_events,
            flow::commands::get_profile,
            flow::commands::save_profile,
            flow::commands::approve_profile,
            flow::commands::start_preview,
            flow::commands::stop_preview,
            flow::commands::open_preview,
            flow::commands::preview_snapshot,
            flow::commands::list_previews,
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
