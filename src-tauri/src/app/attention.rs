//! Atenção do humano: contador no ícone da bandeja e notificação do sistema.
//!
//! O front decide quando alguma coisa pede o usuário; aqui só se mostra isso fora da
//! janela. Os textos são curtos e nunca carregam saída de agente nem credenciais.

use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

use super::tray::TRAY_ID;
use crate::error::{AppError, Result};

const MAX_TITLE_CHARS: usize = 80;
const MAX_BODY_CHARS: usize = 200;
const BASE_TITLE: &str = "H";

fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Texto ao lado do ícone: "H" sem pendências, "H 3" com três tarefas pedindo o usuário. Uma
/// instância de teste leva a etiqueta ("H·dogfood 3"), para não confundi-la com a principal.
pub fn tray_title(count: u32, instance: Option<&str>) -> String {
    let base = match instance {
        Some(tag) => format!("{BASE_TITLE}·{tag}"),
        None => BASE_TITLE.to_owned(),
    };
    if count == 0 {
        base
    } else {
        format!("{base} {}", count.min(99))
    }
}

/// Mostra quantas tarefas precisam do usuário no ícone da bandeja.
#[tauri::command]
pub fn set_attention_count(app: AppHandle, count: u32) -> Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tag = super::instance_tag(&app.config().identifier);
        let _ = tray.set_title(Some(tray_title(count, tag.as_deref())));
        let tooltip = match count {
            0 => "Harnessy — nada pede você agora".to_owned(),
            1 => "Harnessy — 1 tarefa pede você".to_owned(),
            n => format!("Harnessy — {n} tarefas pedem você"),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    Ok(())
}

/// Notificação do sistema. Sem permissão do usuário, não faz nada (e não insiste).
#[tauri::command]
pub fn notify_attention(app: AppHandle, title: String, body: String) -> Result<()> {
    let notification = app.notification();
    let granted = matches!(
        notification.permission_state(),
        Ok(PermissionState::Granted)
    ) || matches!(
        notification.request_permission(),
        Ok(PermissionState::Granted)
    );
    if !granted {
        return Ok(());
    }
    notification
        .builder()
        .title(clip(&title, MAX_TITLE_CHARS))
        .body(clip(&body, MAX_BODY_CHARS))
        .show()
        .map_err(|_| AppError::Validation("Não foi possível mostrar a notificação."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tray_title_counts_up_to_99() {
        assert_eq!(tray_title(0, None), "H");
        assert_eq!(tray_title(3, None), "H 3");
        assert_eq!(tray_title(500, None), "H 99");
        assert_eq!(tray_title(0, Some("dogfood")), "H·dogfood");
        assert_eq!(tray_title(2, Some("dogfood")), "H·dogfood 2");
    }

    #[test]
    fn notification_text_is_clipped_on_char_boundaries() {
        assert_eq!(clip("áéíóú", 3), "áéí");
        assert_eq!(clip("curto", 80), "curto");
    }
}
