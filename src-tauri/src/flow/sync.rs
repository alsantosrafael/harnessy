//! Regras que ligam a run ao card e ao bloqueio, sem custo de tokens.
//!
//! O agente não sabe que existe card: o estado vem de fatos que o Harnessy já
//! tem (estado da run, checks, commit, resumo final e ações negadas pelo
//! `claude`). Tudo aqui é lógica sobre `&Connection`, testável sem Tauri; os
//! adaptadores em `commands.rs` só emitem os eventos.

use rusqlite::Connection;

use crate::{
    error::{AppError, Result},
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
        run::NO_CHANGES | run::FAILED => (resting(db, run)?, run.blocked.as_str()),
        run::CANCELLED => (resting(db, run)?, NOTE_CANCELLED),
        run::INTERRUPTED => (resting(db, run)?, NOTE_INTERRUPTED),
        _ => return Ok(None),
    };
    task::transition(db, run.task_id, to, Actor::System, note)
}

/// Onde o card descansa quando a run não entregou nada: em Revisão se a tarefa já
/// tem trabalho pronto de uma run anterior (um ajuste que falha não desfaz isso),
/// senão em A fazer.
fn resting(db: &Connection, run: &Run) -> Result<Status> {
    Ok(if run::has_completed_besides(db, run.task_id, run.id)? {
        Status::Review
    } else {
        Status::Todo
    })
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
        // Run de escrita grava o revisor ao nascer; a de leitura não. Vale também para Ação
        // de leitura do repositório, que não está na lista fixa `read_only`.
        if read_only.contains(&interrupted.action) || interrupted.reviewer.is_none() {
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

/// Uma run pode ser revisada à mão se terminou com alterações commitadas numa
/// branch e nenhuma outra run está em andamento.
pub fn ensure_reviewable(db: &Connection, run: &Run) -> Result<()> {
    let has_commit = run.branch.is_some() && run.base_sha.is_some();
    if run.state != run::COMPLETED || !has_commit {
        return Err(AppError::Validation(
            "Só uma run concluída, com alterações na branch, pode ser revisada.",
        ));
    }
    if !run::active_runs(db)?.is_empty() {
        return Err(AppError::Validation(
            "Há uma run em andamento. Espere terminar para revisar.",
        ));
    }
    Ok(())
}

/// Exclui a tarefa com as runs dela. Recusa enquanto houver run em andamento:
/// o processo do agente ainda escreve no banco.
pub fn remove_task(db: &Connection, workspace_id: i64, task_id: i64) -> Result<()> {
    task::get(db, workspace_id, task_id)?;
    if run::has_active(db, task_id)? {
        return Err(AppError::Validation(
            "A tarefa tem uma run em andamento. Cancele a run antes de excluir.",
        ));
    }
    let tx = db.unchecked_transaction()?;
    run::delete_for_task(&tx, task_id)?;
    task::delete(&tx, workspace_id, task_id)?;
    tx.commit()?;
    Ok(())
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
    fn a_failed_adjustment_keeps_the_card_in_review_when_work_already_exists() {
        let temp = TempDatabase::new("sync-rest");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (empty_task, empty_run) = fixture(&db);
        // Sem trabalho pronto, uma run vazia devolve o card para A fazer.
        let empty = finish(&db, &empty_run, run::NO_CHANGES, "nada", &[]);
        reconcile(&db, &empty).unwrap();
        assert_eq!(status(&db, empty_task), Status::Todo);

        let task_id = add_task(&db, "Com trabalho");
        let first = run::create(
            &db,
            &NewRun {
                workspace_id: 1,
                task_id,
                title: "Com trabalho",
                action: "implement",
                role: "implementador",
                runtime: "claude",
                model: "sonnet",
                effort: None,
                base_run_id: None,
            },
        )
        .unwrap();
        let done = finish(&db, &first, run::COMPLETED, "ok", &[]);
        reconcile(&db, &done).unwrap();
        assert_eq!(status(&db, task_id), Status::Review);

        // Um ajuste posterior que falha, não muda nada ou é cancelado não desfaz a Revisão.
        for end in [
            run::FAILED,
            run::NO_CHANGES,
            run::CANCELLED,
            run::INTERRUPTED,
        ] {
            let adjust = run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id,
                    title: "Tarefa",
                    action: "implement",
                    role: "implementador",
                    runtime: "claude",
                    model: "sonnet",
                    effort: None,
                    base_run_id: Some(first.id),
                },
            )
            .unwrap();
            reconcile(&db, &adjust).unwrap();
            assert_eq!(status(&db, task_id), Status::Doing);
            let ended = finish(&db, &adjust, end, "x", &[]);
            reconcile(&db, &ended).unwrap();
            assert_eq!(status(&db, task_id), Status::Review, "{end}");
        }
    }

    #[test]
    fn worktrees_left_by_a_task_are_listed_before_its_runs_are_deleted() {
        let temp = TempDatabase::new("sync-worktrees");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        assert!(run::worktrees_of_task(&db, task_id).unwrap().is_empty());
        run::set_worktree(&db, run.id, "abc", "harnessy/run-1-x", "/tmp/wt-1").unwrap();
        finish(&db, &run, run::FAILED, "x", &[]);
        assert_eq!(
            run::worktrees_of_task(&db, task_id).unwrap(),
            vec!["/tmp/wt-1".to_owned()]
        );
        remove_task(&db, 1, task_id).unwrap();
        assert!(run::worktrees_of_task(&db, task_id).unwrap().is_empty());
    }

    #[test]
    fn only_a_completed_run_with_a_branch_is_reviewable_and_never_during_another_run() {
        let temp = TempDatabase::new("sync-reviewable");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (_, run) = fixture(&db);
        assert!(ensure_reviewable(&db, &run).is_err(), "ainda em andamento");

        let completed = finish(&db, &run, run::COMPLETED, "ok", &[]);
        assert!(ensure_reviewable(&db, &completed).is_err(), "sem branch");

        db.execute(
            "UPDATE runs SET branch='harnessy/run-1', base_sha='abc' WHERE id=?1",
            [run.id],
        )
        .unwrap();
        let ready = run::get(&db, run.id).unwrap();
        ensure_reviewable(&db, &ready).unwrap();

        let (task, _) = (add_task(&db, "Outra"), ());
        run::create(
            &db,
            &NewRun {
                workspace_id: 1,
                task_id: task,
                title: "Outra",
                action: "implement",
                role: "implementador",
                runtime: "claude",
                model: "sonnet",
                effort: None,
                base_run_id: None,
            },
        )
        .unwrap();
        assert!(ensure_reviewable(&db, &ready).is_err(), "outra run ativa");
    }

    #[test]
    fn a_task_with_an_active_run_cannot_be_deleted_and_a_finished_one_takes_its_runs_along() {
        let temp = TempDatabase::new("sync-delete");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, run) = fixture(&db);
        let other = add_task(&db, "Outra");
        db.execute("UPDATE tasks SET after_id=?1 WHERE id=?2", [task_id, other])
            .unwrap();
        run::append_event(&db, run.id, "status", "x").unwrap();

        assert!(remove_task(&db, 1, task_id).is_err(), "run ainda ativa");
        assert!(task::get(&db, 1, task_id).is_ok());

        finish(&db, &run, run::COMPLETED, "ok", &[]);
        remove_task(&db, 1, task_id).unwrap();
        assert!(task::get(&db, 1, task_id).is_err());
        assert!(run::get(&db, run.id).is_err());
        assert!(task::history(&db, task_id).unwrap().is_empty());
        let events: i64 = db
            .query_row("SELECT COUNT(*) FROM run_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(events, 0);
        assert!(task::get(&db, 1, other).is_ok(), "a outra tarefa fica");
        assert!(remove_task(&db, 1, task_id).is_err(), "já não existe");
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
        run::set_review_choice(&db, run.id, "claude", "", "auto", None).unwrap();
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
    fn recover_also_spares_a_repository_read_action_missing_from_the_fixed_list() {
        let temp = TempDatabase::new("sync-recover-repo-read");
        let store = temp.open();
        let db = store.connection().unwrap();
        let (task_id, first) = fixture(&db);
        run::set_state(&db, first.id, run::COMPLETED, Some("ok")).unwrap();
        task::transition(&db, task_id, Status::Review, Actor::System, "").unwrap();
        // Ação de leitura do repositório: não está na lista fixa e não tem revisor.
        run::create(
            &db,
            &NewRun {
                workspace_id: 1,
                task_id,
                title: "Tarefa",
                action: "auditar",
                role: "implementador",
                runtime: "claude",
                model: "haiku",
                effort: None,
                base_run_id: None,
            },
        )
        .unwrap();
        assert!(recover(&db, &[]).unwrap().is_empty());
        assert_eq!(status(&db, task_id), Status::Review, "o card não se mexe");
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
