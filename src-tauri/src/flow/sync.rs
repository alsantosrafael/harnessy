//! Regras que ligam a run ao card e ao bloqueio, sem custo de tokens.
//!
//! O agente não sabe que existe card: o estado vem de fatos que o Harnessy já
//! tem (estado da run, checks, commit, resumo final e ações negadas pelo
//! `claude`). Tudo aqui é lógica sobre `&Connection`, testável sem Tauri; os
//! adaptadores em `commands.rs` só emitem os eventos.

use rusqlite::Connection;

use crate::{
    error::Result,
    run::{self, Run},
    task::{self, Actor, Change, Status},
};

const MAX_BLOCKED_BYTES: usize = 600;
const SHOWN_DENIALS: usize = 3;
/// Cada ação negada cabe numa linha; o comando inteiro continua no log da run.
const DENIAL_BYTES: usize = 100;
const NOTE_CANCELLED: &str = "Run cancelada.";
const NOTE_INTERRUPTED: &str = "Run interrompida: o Harnessy foi fechado.";
const NOTE_RESOLVED: &str = "Bloqueio resolvido.";

/// Por que a run ficou presa, em uma linha, ou vazio se não ficou.
///
/// Bloqueio é: a run falhou, terminou sem mudar nada, ou o agente tentou algo
/// que as permissões negam. Cancelar e fechar o app são escolhas, não bloqueios.
pub fn blocker(state: &str, summary: &str, denied: &[String]) -> String {
    let summary = summary.trim();
    let mut parts = Vec::new();
    match state {
        run::FAILED if summary.is_empty() => parts.push("Run falhou.".to_owned()),
        run::FAILED => parts.push(format!("Run falhou: {summary}")),
        run::NO_CHANGES if summary.is_empty() => parts.push("Sem alterações.".to_owned()),
        run::NO_CHANGES => parts.push(format!("Sem alterações: {summary}")),
        run::COMPLETED => {}
        _ => return String::new(),
    }
    if !denied.is_empty() {
        let shown = denied
            .iter()
            .take(SHOWN_DENIALS)
            .map(|denial| shorten(denial, DENIAL_BYTES))
            .collect::<Vec<_>>()
            .join("; ");
        let more = denied.len().saturating_sub(SHOWN_DENIALS);
        let extra = if more > 0 {
            format!(" (+{more})")
        } else {
            String::new()
        };
        parts.push(format!("Ações negadas: {shown}{extra}"));
    }
    clip(&parts.join(" · "), MAX_BLOCKED_BYTES)
}

/// Leva o card da tarefa da run para a raia que o estado da run pede.
/// Nunca reabre uma tarefa concluída; Concluído fica com o usuário ou o revisor.
pub fn reconcile(db: &Connection, run: &Run) -> Result<Option<Change>> {
    if task::get(db, run.workspace_id, run.task_id)?.status == Status::Done {
        return Ok(None);
    }
    let (to, note) = match run.state.as_str() {
        run::PREPARING | run::IMPLEMENTING => (Status::Doing, ""),
        run::COMPLETED if run.blocked.is_empty() => (Status::Review, run.summary.as_str()),
        run::COMPLETED => (Status::Review, run.blocked.as_str()),
        run::NO_CHANGES | run::FAILED => (Status::Todo, run.blocked.as_str()),
        run::CANCELLED => (Status::Todo, NOTE_CANCELLED),
        run::INTERRUPTED => (Status::Todo, NOTE_INTERRUPTED),
        _ => return Ok(None),
    };
    task::transition(db, run.task_id, to, Actor::System, note)
}

/// Na abertura do app: runs que ficaram ativas viram `interrupted` e seus cards
/// saem de Em andamento. Runs de Ações de leitura (`read_only`) não mexem no card.
/// Devolve as mudanças aplicadas.
pub fn recover(db: &Connection, read_only: &[String]) -> Result<Vec<Change>> {
    let active = run::active_runs(db)?;
    run::mark_interrupted(db)?;
    let mut changes = Vec::new();
    for (run_id, _, _) in active {
        let interrupted = run::get(db, run_id)?;
        if read_only.contains(&interrupted.action) {
            continue;
        }
        changes.extend(reconcile(db, &interrupted)?);
    }
    Ok(changes)
}

/// O usuário deu o bloqueio por resolvido: limpa o selo e registra no histórico.
/// Devolve a run atualizada e a mudança do histórico (`None` se já estava limpo).
pub fn resolve(db: &Connection, run_id: i64) -> Result<(Run, Option<Change>)> {
    let run = run::get(db, run_id)?;
    if run.blocked.is_empty() {
        return Ok((run, None));
    }
    run::set_blocked(db, run_id, "")?;
    let status = task::get(db, run.workspace_id, run.task_id)?.status;
    let change = task::transition(db, run.task_id, status, Actor::User, NOTE_RESOLVED)?;
    Ok((run::get(db, run_id)?, change))
}

/// Corta em `max` bytes e marca o corte; uma linha só, sem quebras.
fn shorten(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.len() <= max {
        return flat;
    }
    format!("{}…", clip(&flat, max).trim_end())
}

fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::test_support::TempDatabase, run::NewRun, task::TaskInput};

    fn add_task(db: &Connection, title: &str) -> i64 {
        task::save(
            db,
            &TaskInput {
                workspace_id: 1,
                id: None,
                revision: None,
                title: title.into(),
                description: String::new(),
                criteria: String::new(),
            },
        )
        .unwrap()
        .id
    }

    /// Workspace 1 com uma tarefa e uma run recém-criada (`preparing`) para ela.
    fn fixture(db: &Connection) -> (i64, Run) {
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        let task_id = add_task(db, "Tarefa");
        let run = run::create(
            db,
            &NewRun {
                workspace_id: 1,
                task_id,
                title: "Tarefa",
                action: "implement",
                role: "implementador",
                runtime: "claude",
                model: "sonnet",
                effort: None,
                base_run_id: None,
            },
        )
        .unwrap();
        (task_id, run)
    }

    /// Termina a run como o `AppSink` faz: bloqueio calculado antes do estado.
    fn finish(db: &Connection, run: &Run, state: &str, summary: &str, denied: &[&str]) -> Run {
        let denied: Vec<String> = denied.iter().map(|d| d.to_string()).collect();
        run::set_blocked(db, run.id, &blocker(state, summary, &denied)).unwrap();
        run::set_state(db, run.id, state, Some(summary)).unwrap();
        run::get(db, run.id).unwrap()
    }

    fn status(db: &Connection, task_id: i64) -> Status {
        task::get(db, 1, task_id).unwrap().status
    }

    #[test]
    fn blocker_marks_failures_empty_runs_and_denials_only() {
        assert_eq!(blocker(run::COMPLETED, "ok", &[]), "");
        assert_eq!(blocker(run::CANCELLED, "x", &["Bash: ls".into()]), "");
        assert_eq!(blocker(run::INTERRUPTED, "", &[]), "");
        assert_eq!(blocker(run::FAILED, "", &[]), "Run falhou.");
        assert_eq!(
            blocker(run::FAILED, "Tempo limite da run excedido.", &[]),
            "Run falhou: Tempo limite da run excedido."
        );
        assert_eq!(
            blocker(run::NO_CHANGES, "Preciso de acesso ao banco.", &[]),
            "Sem alterações: Preciso de acesso ao banco."
        );
        assert_eq!(
            blocker(run::COMPLETED, "ok", &["Bash: git push".into()]),
            "Ações negadas: Bash: git push"
        );
        let many: Vec<String> = (1..=5).map(|n| format!("Bash: c{n}")).collect();
        assert_eq!(
            blocker(run::COMPLETED, "", &many),
            "Ações negadas: Bash: c1; Bash: c2; Bash: c3 (+2)"
        );
        assert!(blocker(run::FAILED, &"x".repeat(2_000), &[]).len() <= MAX_BLOCKED_BYTES);

        let long = format!(
            "Bash: cd \"/a/b\" && cat AGENTS.md;\n ls -la; {}",
            "x".repeat(300)
        );
        let reason = blocker(run::COMPLETED, "", &[long]);
        assert!(reason.starts_with("Ações negadas: Bash: cd \"/a/b\" && cat AGENTS.md; ls -la;"));
        assert!(reason.ends_with('…') && !reason.contains('\n') && reason.len() < 160);
    }

    #[test]
    fn run_states_move_the_card_and_the_summary_becomes_the_note() {
        let temp = TempDatabase::new("sync-happy");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);

        reconcile(&db, &run).unwrap();
        assert_eq!(status(&db, task_id), Status::Doing);
        let implementing = finish(&db, &run, run::IMPLEMENTING, "", &[]);
        assert!(
            reconcile(&db, &implementing).unwrap().is_none(),
            "já está lá"
        );

        let completed = finish(&db, &run, run::COMPLETED, "Troquei o rótulo.", &[]);
        let change = reconcile(&db, &completed).unwrap().unwrap();
        assert_eq!(change.status, Status::Review);
        assert_eq!(change.event.actor, "system");
        assert_eq!(change.event.note, "Troquei o rótulo.");
        assert!(completed.blocked.is_empty());
    }

    #[test]
    fn denials_block_a_completed_run_without_hiding_the_card_in_review() {
        let temp = TempDatabase::new("sync-denied");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        reconcile(&db, &run).unwrap();

        let completed = finish(&db, &run, run::COMPLETED, "Feito.", &["Bash: git push"]);
        assert_eq!(completed.blocked, "Ações negadas: Bash: git push");
        let change = reconcile(&db, &completed).unwrap().unwrap();
        assert_eq!(change.status, Status::Review);
        assert_eq!(change.event.note, "Ações negadas: Bash: git push");
        assert_eq!(status(&db, task_id), Status::Review);
    }

    #[test]
    fn failed_and_empty_runs_go_back_to_todo_with_the_reason() {
        let temp = TempDatabase::new("sync-failed");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        reconcile(&db, &run).unwrap();

        let no_changes = finish(&db, &run, run::NO_CHANGES, "Falta a chave da API.", &[]);
        let change = reconcile(&db, &no_changes).unwrap().unwrap();
        assert_eq!(change.status, Status::Todo);
        assert_eq!(change.event.note, "Sem alterações: Falta a chave da API.");
        assert_eq!(status(&db, task_id), Status::Todo);

        task::transition(&db, task_id, Status::Doing, Actor::User, "").unwrap();
        let failed = Run {
            state: run::FAILED.into(),
            blocked: blocker(run::FAILED, "Testes falham.", &[]),
            ..no_changes
        };
        let change = reconcile(&db, &failed).unwrap().unwrap();
        assert_eq!(change.status, Status::Todo);
        assert_eq!(change.event.note, "Run falhou: Testes falham.");
    }

    #[test]
    fn done_is_never_reopened_and_cancel_is_not_a_block() {
        let temp = TempDatabase::new("sync-done");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        reconcile(&db, &run).unwrap();

        let cancelled = finish(&db, &run, run::CANCELLED, "Run cancelada.", &["Bash: ls"]);
        assert!(cancelled.blocked.is_empty());
        let change = reconcile(&db, &cancelled).unwrap().unwrap();
        assert_eq!(change.status, Status::Todo);
        assert_eq!(change.event.note, NOTE_CANCELLED);

        task::transition(&db, task_id, Status::Done, Actor::User, "").unwrap();
        let failed = Run {
            state: run::FAILED.into(),
            blocked: "Run falhou.".into(),
            ..cancelled
        };
        assert!(reconcile(&db, &failed).unwrap().is_none());
        assert_eq!(status(&db, task_id), Status::Done);
    }

    #[test]
    fn recover_interrupts_active_runs_and_frees_their_cards() {
        let temp = TempDatabase::new("sync-recover");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        reconcile(&db, &finish(&db, &run, run::IMPLEMENTING, "", &[])).unwrap();
        assert_eq!(status(&db, task_id), Status::Doing);

        let changes = recover(&db, &[]).unwrap();
        assert_eq!(changes.len(), 1);
        let after = run::get(&db, run.id).unwrap();
        assert_eq!(after.state, run::INTERRUPTED);
        assert!(after.blocked.is_empty(), "fechar o app não é bloqueio");
        assert_eq!(status(&db, task_id), Status::Todo);
        assert_eq!(
            task::history(&db, task_id).unwrap()[0].note,
            NOTE_INTERRUPTED
        );
        assert!(recover(&db, &[]).unwrap().is_empty(), "idempotente");
    }

    #[test]
    fn recover_leaves_the_card_of_a_read_run_alone() {
        let temp = TempDatabase::new("sync-recover-read");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, first) = fixture(&db);
        run::set_state(&db, first.id, run::COMPLETED, Some("ok")).unwrap();
        task::transition(&db, task_id, Status::Review, Actor::System, "").unwrap();
        let read = run::create(
            &db,
            &NewRun {
                workspace_id: 1,
                task_id,
                title: "Tarefa",
                action: "explicar",
                role: "implementador",
                runtime: "claude",
                model: "haiku",
                effort: None,
                base_run_id: None,
            },
        )
        .unwrap();
        assert!(recover(&db, &["explicar".to_owned()]).unwrap().is_empty());
        assert_eq!(run::get(&db, read.id).unwrap().state, run::INTERRUPTED);
        assert_eq!(status(&db, task_id), Status::Review);
    }

    #[test]
    fn resolving_clears_the_block_and_is_logged_once() {
        let temp = TempDatabase::new("sync-resolve");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        let failed = finish(&db, &run, run::FAILED, "Testes falham.", &[]);
        reconcile(&db, &failed).unwrap();
        assert!(!run::get(&db, run.id).unwrap().blocked.is_empty());

        let (updated, change) = resolve(&db, run.id).unwrap();
        assert!(updated.blocked.is_empty());
        let change = change.unwrap();
        assert_eq!(change.event.actor, "user");
        assert_eq!(change.event.note, NOTE_RESOLVED);
        assert_eq!(change.status, Status::Todo, "não move o card");

        let (_, again) = resolve(&db, run.id).unwrap();
        assert!(again.is_none());
        assert_eq!(task::history(&db, task_id).unwrap().len(), 3);
    }
}
