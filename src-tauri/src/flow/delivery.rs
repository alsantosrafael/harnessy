//! Entrega: publicar a branch de uma run como PR, acompanhá-lo e fazer o merge.
//!
//! O agente não participa. As regras daqui são o que o ADR 0001 promete: o diff é
//! verificado antes do push, o PR nasce em draft, nada é forçado e o merge exige
//! que o commit visto pelo usuário seja o que será mesclado.

use crate::{
    db::Database,
    delivery::{self, Delivery, PrText},
    error::{AppError, Result},
    run::{self, publish::Tools, worktree, Run},
    task::{self, Actor, Change, Status},
    workspace,
};

/// Arquivos que nunca devem sair da máquina pelo Harnessy, mesmo se o agente os tocou.
const SENSITIVE: [&str; 6] = [
    ".env",
    "id_rsa",
    "id_ed25519",
    ".pem",
    ".p12",
    "credentials",
];
const NOTE_MERGED: &str = "PR mergeado.";

/// Arquivos do diff `base..branch`, a partir das linhas `diff --git a/x b/x`.
pub(super) fn changed_files(diff: &str) -> Vec<&str> {
    diff.lines()
        .filter_map(|line| line.strip_prefix("diff --git a/"))
        .filter_map(|rest| rest.split(" b/").next())
        .collect()
}

fn sensitive<'a>(files: &[&'a str]) -> Option<&'a str> {
    files.iter().copied().find(|file| {
        let name = file.rsplit('/').next().unwrap_or(file);
        SENSITIVE.iter().any(|mark| name.contains(mark))
    })
}

fn publishable(run: &Run) -> Result<(&str, &str)> {
    if run.state != run::COMPLETED {
        return Err(AppError::Validation(
            "Só uma run concluída pode ser publicada.",
        ));
    }
    match (run.branch.as_deref(), run.base_sha.as_deref()) {
        (Some(branch), Some(base)) => Ok((branch, base)),
        _ => Err(AppError::Validation("A run não deixou uma branch.")),
    }
}

/// Publica a run: verifica o diff, envia a branch e abre (ou adota) o PR em draft.
pub fn publish(
    store: &Database,
    tools: &Tools<'_>,
    git_bin: &std::path::Path,
    run_id: i64,
) -> Result<Delivery> {
    let (run, existing) = {
        let db = store.connection()?;
        (run::get(&db, run_id)?, delivery::get(&db, run_id)?)
    };
    if let Some(existing) = existing {
        return refresh(store, tools, existing.run_id).map(|(d, _)| d);
    }
    let (branch, base) = publishable(&run)?;
    let diff = worktree::diff_range(git_bin, tools.dirs, tools.repo, base, branch, 0)
        .map_err(AppError::Delivery)?;
    let files = changed_files(&diff);
    if files.is_empty() {
        return Err(AppError::Validation(
            "A branch não tem alterações para publicar.",
        ));
    }
    if let Some(file) = sensitive(&files) {
        return Err(AppError::Delivery(format!(
            "Publicação recusada: o diff inclui “{file}”, que parece conter segredos."
        )));
    }
    let task = task::get(&*store.connection()?, run.workspace_id, run.task_id)?;
    tools.push(branch)?;
    let snapshot = match tools.find_pr(branch)? {
        Some(json) => delivery::parse_pr(&json)?,
        None => {
            let stat = files.join("\n");
            let proof = run.proof.as_deref().map(proof_line).unwrap_or_default();
            let review = run.review.as_deref().map(review_line).unwrap_or_default();
            let body = delivery::pr_body(&PrText {
                task_title: &task.title,
                description: &task.description,
                criteria: &task.criteria,
                run_id: run.id,
                summary: &run.summary,
                proof: &proof,
                review: &review,
                diff_stat: &stat,
            });
            tools.create_pr(branch, &delivery::pr_title(&task.title), &body, true)?;
            let json = tools.find_pr(branch)?.ok_or_else(|| {
                AppError::Delivery("O PR foi criado, mas não apareceu no GitHub.".into())
            })?;
            delivery::parse_pr(&json)?
        }
    };
    let db = store.connection()?;
    let saved = delivery::save(&db, run_id, &snapshot)?;
    // A revisão local aprovou este commit: ela continua valendo para o PR.
    if run.review.as_deref() == Some("approve") {
        if let Ok(sha) = worktree::branch_sha(git_bin, tools.dirs, tools.repo, branch) {
            if sha == snapshot.head_sha {
                delivery::set_reviewed(&db, run_id, &sha)?;
            }
        }
    }
    Ok(delivery::get(&db, run_id)?.unwrap_or(saved))
}

fn proof_line(verdict: &str) -> String {
    match verdict {
        "proved" => "Contraprova: os testes novos falham sem a mudança.",
        "not_proved" => "Contraprova: os testes novos passam mesmo sem a mudança.",
        "no_tests" => "Contraprova: a run não criou testes.",
        _ => "",
    }
    .to_owned()
}

fn review_line(verdict: &str) -> String {
    match verdict {
        "approve" => "Revisão automática: aprovada.",
        "request_changes" => "Revisão automática: pediu mudanças.",
        _ => "",
    }
    .to_owned()
}

/// Relê o PR no GitHub e grava o estado. Se ele foi mesclado, a tarefa vai para Concluído.
pub fn refresh(
    store: &Database,
    tools: &Tools<'_>,
    run_id: i64,
) -> Result<(Delivery, Option<Change>)> {
    let current = delivery::get(&*store.connection()?, run_id)?
        .ok_or(AppError::Validation("Esta run ainda não foi publicada."))?;
    let snapshot = delivery::parse_pr(&tools.view_pr(current.pr_number)?)?;
    let db = store.connection()?;
    let saved = delivery::save(&db, run_id, &snapshot)?;
    let change = if snapshot.state == "merged" {
        close_task(&db, run_id)?
    } else {
        None
    };
    Ok((saved, change))
}

fn close_task(db: &rusqlite::Connection, run_id: i64) -> Result<Option<Change>> {
    let run = run::get(db, run_id)?;
    task::transition(db, run.task_id, Status::Done, Actor::System, NOTE_MERGED)
}

/// Por que o merge não deve acontecer agora, ou `None` se está liberado.
pub fn merge_blocker(delivery: &Delivery, review: Option<&str>) -> Option<&'static str> {
    if delivery.state != "open" {
        return Some("O PR não está aberto.");
    }
    if delivery.checks == "failing" {
        return Some("Há checks reprovados no PR.");
    }
    if delivery.checks == "pending" {
        return Some("Os checks do PR ainda estão rodando.");
    }
    if delivery.mergeable == "conflicting" {
        return Some("O PR tem conflitos com a base.");
    }
    if review == Some("request_changes") && !delivery.review_is_current() {
        return Some("A revisão pediu mudanças e nada foi corrigido desde então.");
    }
    None
}

/// Merge (squash) do PR depois de reler o estado. O commit mesclado é o que acabou de ser lido.
pub fn merge(
    store: &Database,
    tools: &Tools<'_>,
    run_id: i64,
) -> Result<(Delivery, Option<Change>)> {
    let (fresh, _) = refresh(store, tools, run_id)?;
    let run = run::get(&*store.connection()?, run_id)?;
    if let Some(reason) = merge_blocker(&fresh, run.review.as_deref()) {
        return Err(AppError::Delivery(reason.to_owned()));
    }
    let sha = fresh
        .head_sha
        .clone()
        .ok_or_else(|| AppError::Delivery("O GitHub não informou o commit do PR.".into()))?;
    if fresh.draft {
        tools.mark_ready(fresh.pr_number)?;
    }
    tools.merge(fresh.pr_number, &sha)?;
    refresh(store, tools, run_id)
}

pub fn list(db: &rusqlite::Connection, workspace_id: i64) -> Result<Vec<Delivery>> {
    // Valida que o workspace existe antes de listar.
    workspace::get(db, workspace_id)?;
    delivery::list(db, workspace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delivery(state: &str, checks: &str, mergeable: &str) -> Delivery {
        Delivery {
            run_id: 1,
            pr_number: 3,
            pr_url: "u".into(),
            state: state.into(),
            draft: true,
            head_sha: Some("a".into()),
            checks: checks.into(),
            mergeable: mergeable.into(),
            reviewed_sha: None,
            published_at: 0,
            merged_at: None,
        }
    }

    #[test]
    fn lists_files_and_flags_secrets() {
        let diff = "diff --git a/src/a.ts b/src/a.ts\n+x\ndiff --git a/.env.local b/.env.local\n+k";
        let files = changed_files(diff);
        assert_eq!(files, ["src/a.ts", ".env.local"]);
        assert_eq!(sensitive(&files), Some(".env.local"));
        assert_eq!(sensitive(&files[..1]), None);
    }

    #[test]
    fn merge_is_blocked_until_the_pr_is_safe() {
        assert!(merge_blocker(&delivery("open", "passing", "mergeable"), None).is_none());
        assert!(merge_blocker(&delivery("open", "none", "unknown"), Some("approve")).is_none());
        assert!(merge_blocker(&delivery("merged", "passing", "mergeable"), None).is_some());
        assert!(merge_blocker(&delivery("open", "failing", "mergeable"), None).is_some());
        assert!(merge_blocker(&delivery("open", "pending", "mergeable"), None).is_some());
        assert!(merge_blocker(&delivery("open", "passing", "conflicting"), None).is_some());
        assert!(merge_blocker(
            &delivery("open", "passing", "mergeable"),
            Some("request_changes")
        )
        .is_some());
    }
}
