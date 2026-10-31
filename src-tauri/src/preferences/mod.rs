//! Preferências pessoais guardadas no banco local (hoje, a escolha de agente e esforço).
//!
//! Antes essa escolha vivia no `localStorage` da WebView, que é separado por origem: o
//! app em desenvolvimento e o instalado não a compartilhavam, e ela se perdia ao limpar o
//! cache. No banco ela é a mesma para qualquer janela e sobrevive a reinstalações.
//!
//! O conteúdo é um documento JSON opaco; quem usa a preferência a valida (por exemplo,
//! `start_run` recusa um agente ou esforço desconhecido).

pub mod commands;

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    db::now,
    error::{AppError, Result},
};

const MAX_VALUE_BYTES: usize = 4 * 1024;
/// Chaves aceitas: nada além do que o app realmente guarda.
const KEYS: [&str; 1] = ["agent-choice"];

fn check_key(key: &str) -> Result<()> {
    if KEYS.contains(&key) {
        Ok(())
    } else {
        Err(AppError::Validation("Preferência desconhecida."))
    }
}

/// Valor guardado, ou `None` se ainda não foi definido.
pub fn get(db: &Connection, key: &str) -> Result<Option<String>> {
    check_key(key)?;
    Ok(db
        .query_row("SELECT value FROM preferences WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

/// Guarda um objeto JSON pequeno; recusa qualquer outra coisa.
pub fn set(db: &Connection, key: &str, value: &str) -> Result<()> {
    check_key(key)?;
    if value.len() > MAX_VALUE_BYTES {
        return Err(AppError::Validation("A preferência é grande demais."));
    }
    let is_object = serde_json::from_str::<serde_json::Value>(value)
        .map(|json| json.is_object())
        .unwrap_or(false);
    if !is_object {
        return Err(AppError::Validation(
            "A preferência deve ser um objeto JSON.",
        ));
    }
    db.execute(
        "INSERT INTO preferences (key, value, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value=?2, updated_at=?3",
        params![key, value, now()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    #[test]
    fn stores_and_replaces_a_json_object_per_key() {
        let temp = TempDatabase::new("prefs");
        let store = temp.open();
        let db = store.connection().unwrap();
        assert_eq!(get(&db, "agent-choice").unwrap(), None);
        set(&db, "agent-choice", r#"{"runtime":"claude"}"#).unwrap();
        set(&db, "agent-choice", r#"{"runtime":"codex"}"#).unwrap();
        assert_eq!(
            get(&db, "agent-choice").unwrap().as_deref(),
            Some(r#"{"runtime":"codex"}"#)
        );
    }

    #[test]
    fn refuses_unknown_keys_non_objects_and_huge_values() {
        let temp = TempDatabase::new("prefs-bad");
        let store = temp.open();
        let db = store.connection().unwrap();
        assert!(set(&db, "outra-coisa", "{}").is_err());
        assert!(get(&db, "outra-coisa").is_err());
        assert!(set(&db, "agent-choice", "[1,2]").is_err());
        assert!(set(&db, "agent-choice", "não é json").is_err());
        let huge = format!(r#"{{"x":"{}"}}"#, "a".repeat(MAX_VALUE_BYTES));
        assert!(set(&db, "agent-choice", &huge).is_err());
    }
}
