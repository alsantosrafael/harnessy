//! Entrega de uma run: o PR que a publica, o estado dele no GitHub e a revisão feita sobre ele.
//!
//! Só modelo, banco e interpretação de texto. Quem executa `git push` e `gh` é
//! `run::publish`, e quem cruza run, tarefa e workspace é `flow`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    db::now,
    error::{AppError, Result},
};

/// Limite do corpo do PR; o GitHub aceita bem mais, mas texto enorme não ajuda quem revisa.
const MAX_BODY_CHARS: usize = 6_000;
const MAX_TITLE_CHARS: usize = 120;

const COLUMNS: &str = "run_id, pr_number, pr_url, state, draft, head_sha, checks, mergeable, \
                       reviewed_sha, published_at, merged_at";

/// PR de uma run, como o Harnessy o conhece. `checks` e `mergeable` vêm do GitHub.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub run_id: i64,
    pub pr_number: i64,
    pub pr_url: String,
    /// `open`, `merged` ou `closed`.
    pub state: String,
    pub draft: bool,
    pub head_sha: Option<String>,
    /// `none`, `pending`, `passing` ou `failing`.
    pub checks: String,
    /// `unknown`, `mergeable` ou `conflicting`.
    pub mergeable: String,
    /// SHA que o revisor aprovou; se o PR avançar, a aprovação deixa de valer.
    pub reviewed_sha: Option<String>,
    pub published_at: i64,
    pub merged_at: Option<i64>,
}

impl Delivery {
    fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            run_id: r.get(0)?,
            pr_number: r.get(1)?,
            pr_url: r.get(2)?,
            state: r.get(3)?,
            draft: r.get::<_, i64>(4)? != 0,
            head_sha: r.get(5)?,
            checks: r.get(6)?,
            mergeable: r.get(7)?,
            reviewed_sha: r.get(8)?,
            published_at: r.get(9)?,
            merged_at: r.get(10)?,
        })
    }

    /// A revisão vale para o que está no PR agora.
    pub fn review_is_current(&self) -> bool {
        matches!((&self.reviewed_sha, &self.head_sha), (Some(a), Some(b)) if a == b)
    }
}

/// Estado do PR lido do GitHub (`gh pr view --json ...`).
#[derive(Debug, Clone, PartialEq)]
pub struct PrSnapshot {
    pub number: i64,
    pub url: String,
    pub state: &'static str,
    pub draft: bool,
    pub head_sha: String,
    pub checks: &'static str,
    pub mergeable: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPr {
    number: i64,
    url: String,
    state: String,
    #[serde(default)]
    is_draft: bool,
    #[serde(default)]
    head_ref_oid: String,
    #[serde(default)]
    status_check_rollup: Vec<RawCheck>,
    #[serde(default)]
    mergeable: String,
}

#[derive(Deserialize, Default)]
struct RawCheck {
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: String,
    #[serde(default)]
    state: String,
}

/// Campos que `gh pr view` precisa devolver para [`parse_pr`].
pub const PR_FIELDS: &str = "number,url,state,isDraft,headRefOid,statusCheckRollup,mergeable";

/// Interpreta o JSON de `gh pr view` ou de um item de `gh pr list`.
pub fn parse_pr(json: &str) -> Result<PrSnapshot> {
    let raw: RawPr = serde_json::from_str(json.trim())
        .map_err(|_| AppError::Delivery("Resposta inesperada do GitHub sobre o PR.".into()))?;
    Ok(PrSnapshot {
        number: raw.number,
        url: raw.url,
        state: match raw.state.as_str() {
            "MERGED" => "merged",
            "CLOSED" => "closed",
            _ => "open",
        },
        draft: raw.is_draft,
        head_sha: raw.head_ref_oid,
        checks: summarize_checks(&raw.status_check_rollup),
        mergeable: match raw.mergeable.as_str() {
            "MERGEABLE" => "mergeable",
            "CONFLICTING" => "conflicting",
            _ => "unknown",
        },
    })
}

fn summarize_checks(checks: &[RawCheck]) -> &'static str {
    const BAD: [&str; 5] = [
        "FAILURE",
        "ERROR",
        "TIMED_OUT",
        "CANCELLED",
        "STARTUP_FAILURE",
    ];
    if checks.is_empty() {
        return "none";
    }
    let failing =
        |c: &RawCheck| BAD.contains(&c.conclusion.as_str()) || BAD.contains(&c.state.as_str());
    if checks.iter().any(failing) {
        return "failing";
    }
    let pending = |c: &RawCheck| {
        (!c.status.is_empty() && c.status != "COMPLETED")
            || c.state == "PENDING"
            || c.state == "EXPECTED"
    };
    if checks.iter().any(pending) {
        "pending"
    } else {
        "passing"
    }
}

/// Dados que entram no texto do PR.
pub struct PrText<'a> {
    pub task_title: &'a str,
    pub description: &'a str,
    pub criteria: &'a str,
    pub run_id: i64,
    pub summary: &'a str,
    pub proof: &'a str,
    pub review: &'a str,
    pub diff_stat: &'a str,
}

/// Título do PR: o da tarefa, em uma linha.
pub fn pr_title(task_title: &str) -> String {
    let line = task_title.lines().next().unwrap_or("").trim();
    let title: String = line.chars().take(MAX_TITLE_CHARS).collect();
    if title.is_empty() {
        "Alterações do Harnessy".to_owned()
    } else {
        title
    }
}

/// Corpo do PR: tarefa, critérios, resultado da run e identificação para reconciliar.
pub fn pr_body(text: &PrText<'_>) -> String {
    let mut body = String::new();
    let mut section = |title: &str, content: &str| {
        let content = content.trim();
        if !content.is_empty() {
            body.push_str(&format!("## {title}\n\n{content}\n\n"));
        }
    };
    section("Tarefa", text.description);
    section("Critérios de aceite", text.criteria);
    section("Resumo da run", text.summary);
    section("Verificações", &format!("{}\n{}", text.proof, text.review));
    if !text.diff_stat.trim().is_empty() {
        section(
            "Arquivos alterados",
            &format!("```\n{}\n```", text.diff_stat.trim()),
        );
    }
    body.push_str(&format!(
        "---\nPublicado pelo Harnessy · run #{} · tarefa “{}”.\n",
        text.run_id,
        pr_title(text.task_title)
    ));
    body.chars().take(MAX_BODY_CHARS).collect()
}

/// Grava o PR da run; chamar de novo atualiza o estado sem duplicar (idempotente).
pub fn save(db: &Connection, run_id: i64, pr: &PrSnapshot) -> Result<Delivery> {
    let merged = (pr.state == "merged").then(now);
    db.execute(
        "INSERT INTO deliveries (run_id, pr_number, pr_url, state, draft, head_sha, checks, mergeable, published_at, merged_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
         ON CONFLICT(run_id) DO UPDATE SET pr_number=?2, pr_url=?3, state=?4, draft=?5, head_sha=?6, \
         checks=?7, mergeable=?8, merged_at=COALESCE(deliveries.merged_at, ?10)",
        params![
            run_id,
            pr.number,
            pr.url,
            pr.state,
            pr.draft,
            pr.head_sha,
            pr.checks,
            pr.mergeable,
            now(),
            merged
        ],
    )?;
    get(db, run_id)?.ok_or(AppError::DatabaseUnavailable)
}

pub fn get(db: &Connection, run_id: i64) -> Result<Option<Delivery>> {
    Ok(db
        .query_row(
            &format!("SELECT {COLUMNS} FROM deliveries WHERE run_id=?1"),
            [run_id],
            Delivery::from_row,
        )
        .optional()?)
}

pub fn list(db: &Connection, workspace_id: i64) -> Result<Vec<Delivery>> {
    let mut query = db.prepare(&format!(
        "SELECT {COLUMNS} FROM deliveries WHERE run_id IN (SELECT id FROM runs WHERE workspace_id=?1) \
         ORDER BY run_id DESC"
    ))?;
    let rows = query.query_map([workspace_id], Delivery::from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Marca o SHA que a revisão aprovou.
pub fn set_reviewed(db: &Connection, run_id: i64, sha: &str) -> Result<()> {
    db.execute(
        "UPDATE deliveries SET reviewed_sha=?2 WHERE run_id=?1",
        params![run_id, sha],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    fn snapshot(state: &'static str, sha: &str) -> PrSnapshot {
        PrSnapshot {
            number: 7,
            url: "https://github.com/a/b/pull/7".into(),
            state,
            draft: true,
            head_sha: sha.into(),
            checks: "none",
            mergeable: "unknown",
        }
    }

    #[test]
    fn parses_pr_view_json() {
        let json = r#"{"number":7,"url":"https://github.com/a/b/pull/7","state":"OPEN","isDraft":true,
            "headRefOid":"abc","mergeable":"MERGEABLE",
            "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"},{"state":"SUCCESS"}]}"#;
        let pr = parse_pr(json).unwrap();
        assert_eq!((pr.number, pr.state, pr.draft), (7, "open", true));
        assert_eq!((pr.checks, pr.mergeable), ("passing", "mergeable"));
    }

    #[test]
    fn check_summary_prefers_failure_then_pending() {
        let check = |status: &str, conclusion: &str| RawCheck {
            status: status.into(),
            conclusion: conclusion.into(),
            state: String::new(),
        };
        assert_eq!(summarize_checks(&[]), "none");
        assert_eq!(
            summarize_checks(&[check("COMPLETED", "FAILURE"), check("IN_PROGRESS", "")]),
            "failing"
        );
        assert_eq!(
            summarize_checks(&[check("COMPLETED", "SUCCESS"), check("IN_PROGRESS", "")]),
            "pending"
        );
    }

    #[test]
    fn merged_and_closed_states_are_mapped() {
        let json =
            |state: &str| format!(r#"{{"number":1,"url":"u","state":"{state}","headRefOid":"x"}}"#);
        assert_eq!(parse_pr(&json("MERGED")).unwrap().state, "merged");
        assert_eq!(parse_pr(&json("CLOSED")).unwrap().state, "closed");
        assert!(parse_pr("não é json").is_err());
    }

    #[test]
    fn body_has_sections_and_run_identity() {
        let body = pr_body(&PrText {
            task_title: "Nova tela",
            description: "Faz a tela.",
            criteria: "- abre",
            run_id: 12,
            summary: "Feito.",
            proof: "Contraprova: provada.",
            review: "",
            diff_stat: " a.ts | 2 +-",
        });
        assert!(body.contains("## Critérios de aceite"));
        assert!(body.contains("run #12"));
        assert!(!body.contains("## Resumo da run\n\n\n"));
        assert_eq!(pr_title("  \n"), "Alterações do Harnessy");
    }

    #[test]
    fn save_is_idempotent_and_review_follows_the_sha() {
        let temp = TempDatabase::new("delivery");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute("PRAGMA foreign_keys=OFF", []).unwrap();
        let first = save(&db, 1, &snapshot("open", "aaa")).unwrap();
        assert!(!first.review_is_current());
        set_reviewed(&db, 1, "aaa").unwrap();
        assert!(get(&db, 1).unwrap().unwrap().review_is_current());
        let moved = save(&db, 1, &snapshot("open", "bbb")).unwrap();
        assert!(
            !moved.review_is_current(),
            "novo commit invalida a aprovação"
        );
        let merged = save(&db, 1, &snapshot("merged", "bbb")).unwrap();
        assert!(merged.merged_at.is_some());
        assert_eq!(list_count(&db), 1);
    }

    fn list_count(db: &Connection) -> i64 {
        db.query_row("SELECT COUNT(*) FROM deliveries", [], |r| r.get(0))
            .unwrap()
    }
}
