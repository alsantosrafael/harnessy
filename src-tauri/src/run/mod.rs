//! Runs: execuções de uma Ação sobre uma Tarefa, com estado e eventos.
//!
//! Este módulo guarda o modelo e as regras de persistência. Os subprocessos do
//! agente ficam em [`supervisor`]; o contrato com o runtime é [`runtime`], e o
//! adaptadores do Claude Code ([`claude`]) e do Codex ([`codex`]) são os únicos que conhecem as CLIs. O que é
//! entregue ao agente (prompt, settings e ambiente) é montado em Rust
//! (regras 25–28).

pub mod claude;
pub mod codex;
pub mod commands;
pub mod guard;
pub mod prompt;
pub mod redact;
pub mod runtime;
mod stream;
pub mod supervisor;
pub mod worktree;

use rusqlite::{params, Connection, Row};
use serde::Serialize;

use runtime::CycleUsage;

use crate::{
    db::now,
    error::{AppError, Result},
};

pub const PREPARING: &str = "preparing";
pub const IMPLEMENTING: &str = "implementing";
pub const COMPLETED: &str = "completed";
pub const NO_CHANGES: &str = "no_changes";
pub const FAILED: &str = "failed";
pub const CANCELLED: &str = "cancelled";
pub const INTERRUPTED: &str = "interrupted";

const TERMINAL: [&str; 5] = [COMPLETED, NO_CHANGES, FAILED, CANCELLED, INTERRUPTED];
/// Texto total de eventos por run (1 MB), para o log não crescer sem limite.
const EVENT_LOG_LIMIT: i64 = 1024 * 1024;
/// Motivos de um ciclo do agente: primeira implementação ou correção após checks.
pub const TRIGGER_INITIAL: &str = "initial";
pub const TRIGGER_CHECK_FAILED: &str = "check_failed";
/// Ciclo pedido pelo usuário com o próximo modelo da lista ([`guard::next_model`]).
pub const TRIGGER_BIGGER_MODEL: &str = "bigger_model";
/// Correção pedida porque nenhum teste prova a mudança (`exigir_prova`).
pub const TRIGGER_PROOF: &str = "proof";
/// Ciclo do revisor e a correção que ele pede.
pub const TRIGGER_REVIEW: &str = "review";
/// Comandos negados lembrados por workspace, entregues no prompt seguinte.
const MAX_DENIALS: usize = 20;
const MAX_DENIAL_BYTES: usize = 80;

const COLUMNS: &str = "id, workspace_id, task_id, title, action, role, runtime, state, \
                       base_sha, branch, worktree_path, started_at, finished_at, summary, blocked, \
                       model, cycles, turns, input_tokens, output_tokens, cache_read_tokens, \
                       cache_write_tokens, cost_usd, agent_ms, checks_ms, proof, effort, review, review_discarded, base_run_id, reviewer, \
                       (SELECT c.model FROM run_cycles c WHERE c.run_id = runs.id \
                        ORDER BY c.cycle DESC LIMIT 1)";

pub fn is_terminal(state: &str) -> bool {
    TERMINAL.contains(&state)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: i64,
    pub workspace_id: i64,
    pub task_id: i64,
    pub title: String,
    pub action: String,
    pub role: String,
    pub runtime: String,
    pub state: String,
    pub base_sha: Option<String>,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub summary: String,
    /// Motivo objetivo de a run ter ficado presa; vazio quando não houve bloqueio.
    pub blocked: String,
    /// Modelo pedido na run (alias do Claude Code).
    pub model: Option<String>,
    /// Ciclos do agente gravados em `run_cycles`.
    pub cycles: Option<i64>,
    pub turns: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    /// Soma dos ciclos; `None` se algum ciclo não informou o custo.
    pub cost_usd: Option<f64>,
    /// Tempo somado do agente e dos checks, em milissegundos.
    pub agent_ms: Option<i64>,
    pub checks_ms: Option<i64>,
    /// Veredito da contraprova (`proved`, `not_proved`, `no_tests`, `skipped`,
    /// `inconclusive`); `None` quando a etapa não rodou.
    pub proof: Option<String>,
    /// Esforço de raciocínio pedido pela Ação; `None` quando não foi definido.
    pub effort: Option<String>,
    /// Veredito do revisor (`approve`, `request_changes`, `invalid`); `None` sem revisão.
    pub review: Option<String>,
    /// Achados do revisor descartados por não apontarem para o diff.
    pub review_discarded: Option<i64>,
    /// Run cuja branch serviu de base (passo encadeado); `None` = ramo base do workspace.
    pub base_run_id: Option<i64>,
    /// Revisor escolhido (`claude` ou `codex`); `None` em runs anteriores a ele, que valem `claude`.
    pub reviewer: Option<String>,
    /// Modelo do último ciclo gravado.
    pub last_model: Option<String>,
    /// Próximo modelo da lista acima do último ciclo; `None` no topo.
    pub next_model: Option<String>,
}

impl Run {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            workspace_id: r.get(1)?,
            task_id: r.get(2)?,
            title: r.get(3)?,
            action: r.get(4)?,
            role: r.get(5)?,
            runtime: r.get(6)?,
            state: r.get(7)?,
            base_sha: r.get(8)?,
            branch: r.get(9)?,
            worktree_path: r.get(10)?,
            started_at: r.get(11)?,
            finished_at: r.get(12)?,
            summary: r.get(13)?,
            blocked: r.get(14)?,
            model: r.get(15)?,
            cycles: r.get(16)?,
            turns: r.get(17)?,
            input_tokens: r.get(18)?,
            output_tokens: r.get(19)?,
            cache_read_tokens: r.get(20)?,
            cache_write_tokens: r.get(21)?,
            cost_usd: r.get(22)?,
            agent_ms: r.get(23)?,
            checks_ms: r.get(24)?,
            proof: r.get(25)?,
            effort: r.get(26)?,
            review: r.get(27)?,
            review_discarded: r.get(28)?,
            base_run_id: r.get(29)?,
            reviewer: r.get(30)?,
            last_model: None,
            next_model: None,
        })
        .and_then(|mut run| {
            run.last_model = r.get::<_, Option<String>>(31)?.or(run.model.clone());
            run.next_model = run
                .last_model
                .as_deref()
                .and_then(guard::next_model)
                .map(str::to_owned);
            Ok(run)
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    pub run_id: i64,
    pub seq: i64,
    pub kind: String,
    pub text: String,
}

/// Dados de uma nova run; `role` e `runtime` vêm de `src/lib/roles.ts`.
pub struct NewRun<'a> {
    pub workspace_id: i64,
    pub task_id: i64,
    pub title: &'a str,
    pub action: &'a str,
    pub role: &'a str,
    pub runtime: &'a str,
    pub model: &'a str,
    pub effort: Option<&'a str>,
    /// Run anterior cuja branch é a base desta.
    pub base_run_id: Option<i64>,
}

/// Um ciclo do agente, do início ao fim, para o ledger. `agent_ms` é o tempo do processo.
pub struct CycleRecord<'a> {
    pub cycle: i64,
    pub model: &'a str,
    pub session_id: Option<&'a str>,
    /// O ciclo retomou a sessão do ciclo anterior.
    pub resumed: bool,
    pub usage: CycleUsage,
    pub trigger: &'a str,
    pub started_at: i64,
    pub finished_at: i64,
    pub agent_ms: i64,
}

/// Cria a run em `preparing`. Só uma run ativa por workspace.
pub fn create(db: &Connection, new: &NewRun<'_>) -> Result<Run> {
    create_guard(db, new.workspace_id)?;
    db.execute(
        "INSERT INTO runs(workspace_id, task_id, title, action, role, runtime, state, started_at, model, effort, base_run_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            new.workspace_id,
            new.task_id,
            new.title,
            new.action,
            new.role,
            new.runtime,
            PREPARING,
            now(),
            new.model,
            new.effort,
            new.base_run_id
        ],
    )?;
    get(db, db.last_insert_rowid())
}

/// Recusa uma nova execução quando o workspace já tem uma run ativa.
fn create_guard(db: &Connection, workspace_id: i64) -> Result<()> {
    let active: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM runs WHERE workspace_id=?1 AND state NOT IN (?2,?3,?4,?5,?6))",
        params![
            workspace_id,
            TERMINAL[0],
            TERMINAL[1],
            TERMINAL[2],
            TERMINAL[3],
            TERMINAL[4]
        ],
        |r| r.get(0),
    )?;
    if active {
        return Err(AppError::Validation(
            "Já existe uma run ativa neste workspace. Aguarde ou cancele.",
        ));
    }
    Ok(())
}

/// Grava um ciclo em `run_cycles` e atualiza os totais em `runs`. Um total só
/// existe se todos os ciclos informaram o campo; caso contrário fica `NULL`.
pub fn record_cycle(db: &Connection, run_id: i64, cycle: &CycleRecord<'_>) -> Result<()> {
    let usage = cycle.usage;
    db.execute(
        "INSERT INTO run_cycles(run_id, cycle, model, session_id, resumed, input_tokens, output_tokens, \
         cache_read_tokens, cache_write_tokens, cost_usd, turns, \"trigger\", started_at, finished_at) \
         VALUES (?1, ?2, ?3, ?4, ?14, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            run_id,
            cycle.cycle,
            cycle.model,
            cycle.session_id,
            int(usage.input),
            int(usage.output),
            int(usage.cache_read),
            int(usage.cache_write),
            usage.cost_usd,
            int(usage.turns),
            cycle.trigger,
            cycle.started_at,
            cycle.finished_at,
            cycle.resumed
        ],
    )?;
    db.execute(
        "UPDATE runs SET agent_ms = COALESCE(agent_ms, 0) + ?2, \
         cycles = (SELECT COUNT(*) FROM run_cycles WHERE run_id=?1), \
         turns = (SELECT CASE WHEN COUNT(turns)=COUNT(*) THEN SUM(turns) END FROM run_cycles WHERE run_id=?1), \
         input_tokens = (SELECT CASE WHEN COUNT(input_tokens)=COUNT(*) THEN SUM(input_tokens) END FROM run_cycles WHERE run_id=?1), \
         output_tokens = (SELECT CASE WHEN COUNT(output_tokens)=COUNT(*) THEN SUM(output_tokens) END FROM run_cycles WHERE run_id=?1), \
         cache_read_tokens = (SELECT CASE WHEN COUNT(cache_read_tokens)=COUNT(*) THEN SUM(cache_read_tokens) END FROM run_cycles WHERE run_id=?1), \
         cache_write_tokens = (SELECT CASE WHEN COUNT(cache_write_tokens)=COUNT(*) THEN SUM(cache_write_tokens) END FROM run_cycles WHERE run_id=?1), \
         cost_usd = (SELECT CASE WHEN COUNT(cost_usd)=COUNT(*) THEN SUM(cost_usd) END FROM run_cycles WHERE run_id=?1) \
         WHERE id=?1",
        params![run_id, cycle.agent_ms],
    )?;
    Ok(())
}

/// Soma o tempo dos checks de uma run.
pub fn add_checks_ms(db: &Connection, run_id: i64, ms: i64) -> Result<()> {
    db.execute(
        "UPDATE runs SET checks_ms = COALESCE(checks_ms, 0) + ?2 WHERE id=?1",
        params![run_id, ms],
    )?;
    Ok(())
}

fn int(value: Option<u64>) -> Option<i64> {
    value.map(|v| v as i64)
}

pub fn get(db: &Connection, id: i64) -> Result<Run> {
    db.query_row(
        &format!("SELECT {COLUMNS} FROM runs WHERE id=?1"),
        [id],
        Run::from_row,
    )
    .map_err(|_| AppError::Validation("Run não encontrada."))
}

pub fn list(db: &Connection, workspace_id: i64) -> Result<Vec<Run>> {
    let mut query = db.prepare(&format!(
        "SELECT {COLUMNS} FROM runs WHERE workspace_id=?1 ORDER BY id DESC LIMIT 50"
    ))?;
    let rows = query.query_map([workspace_id], Run::from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Muda o estado de uma run ainda ativa. Estado terminal nunca é sobrescrito
/// (um cancelamento não vira falha quando o processo morre depois).
pub fn set_state(db: &Connection, id: i64, state: &str, summary: Option<&str>) -> Result<bool> {
    let finished = is_terminal(state).then(now);
    let changed = db.execute(
        "UPDATE runs SET state=?2, finished_at=COALESCE(?3, finished_at), summary=COALESCE(?4, summary) \
         WHERE id=?1 AND state NOT IN (?5,?6,?7,?8,?9)",
        params![
            id, state, finished, summary, TERMINAL[0], TERMINAL[1], TERMINAL[2], TERMINAL[3],
            TERMINAL[4]
        ],
    )?;
    Ok(changed > 0)
}

/// Grava (ou, com texto vazio, limpa) o bloqueio da run.
pub fn set_blocked(db: &Connection, id: i64, reason: &str) -> Result<()> {
    db.execute(
        "UPDATE runs SET blocked=?2 WHERE id=?1",
        params![id, reason],
    )?;
    Ok(())
}

pub fn set_worktree(
    db: &Connection,
    id: i64,
    base_sha: &str,
    branch: &str,
    path: &str,
) -> Result<()> {
    db.execute(
        "UPDATE runs SET base_sha=?2, branch=?3, worktree_path=?4 WHERE id=?1",
        params![id, base_sha, branch, path],
    )?;
    Ok(())
}

/// Reabre uma run que falhou para um ciclo extra pedido pelo usuário. Só vale
/// para `failed` com worktree preservado; devolve a run já em `preparing`.
pub fn reopen(db: &Connection, id: i64) -> Result<Run> {
    let run = get(db, id)?;
    if run.state != FAILED || run.worktree_path.is_none() || run.base_sha.is_none() {
        return Err(AppError::Validation(
            "Só uma run que falhou e manteve a pasta de trabalho pode ganhar outro ciclo.",
        ));
    }
    create_guard(db, run.workspace_id)?;
    db.execute(
        "UPDATE runs SET state=?2, finished_at=NULL, blocked='' WHERE id=?1",
        params![id, PREPARING],
    )?;
    get(db, id)
}

/// Número do último ciclo gravado (0 sem ciclos).
pub fn last_cycle(db: &Connection, id: i64) -> Result<i64> {
    Ok(db.query_row(
        "SELECT COALESCE(MAX(cycle), 0) FROM run_cycles WHERE run_id=?1",
        [id],
        |r| r.get(0),
    )?)
}

/// Chave estável de uma ação negada: `Bash: curl -s x` vira `curl`; `git` e `gh`
/// guardam o subcomando (`git push`); outras ferramentas, o nome (`WebFetch`).
pub fn denial_key(text: &str) -> String {
    let key = match text.strip_prefix("Bash: ") {
        Some(command) => {
            let mut words = command.split_whitespace();
            match words.next() {
                Some(head @ ("git" | "gh" | "cargo" | "bun" | "npm")) => match words.next() {
                    Some(sub) if !sub.starts_with('-') => format!("{head} {sub}"),
                    _ => head.to_owned(),
                },
                Some(head) => head.to_owned(),
                None => "Bash".to_owned(),
            }
        }
        None => text.split(':').next().unwrap_or(text).trim().to_owned(),
    };
    key.chars().take(MAX_DENIAL_BYTES).collect()
}

/// Soma as ações negadas de uma run aos comandos lembrados do workspace.
pub fn record_denials(db: &Connection, workspace_id: i64, denied: &[String]) -> Result<()> {
    let mut keys: Vec<String> = denied.iter().map(|text| denial_key(text)).collect();
    keys.sort();
    keys.dedup();
    for key in keys
        .iter()
        .filter(|key| !key.is_empty() && key.as_str() != "?")
    {
        db.execute(
            "INSERT INTO workspace_denials(workspace_id, command, count, last_seen) VALUES (?1, ?2, 1, ?3) \
             ON CONFLICT(workspace_id, command) DO UPDATE SET count = count + 1, last_seen = ?3",
            params![workspace_id, key, now()],
        )?;
    }
    Ok(())
}

/// Comandos negados mais frequentes do workspace, para o prompt das próximas runs.
pub fn top_denials(db: &Connection, workspace_id: i64) -> Result<Vec<String>> {
    let mut query = db.prepare(
        "SELECT command FROM workspace_denials WHERE workspace_id=?1 \
         ORDER BY count DESC, last_seen DESC LIMIT ?2",
    )?;
    let rows = query.query_map(params![workspace_id, MAX_DENIALS as i64], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Grava o veredito do revisor e quantos achados o filtro descartou.
pub fn set_review(db: &Connection, id: i64, verdict: &str, discarded: usize) -> Result<()> {
    db.execute(
        "UPDATE runs SET review=?2, review_discarded=?3 WHERE id=?1",
        params![id, verdict, discarded as i64],
    )?;
    Ok(())
}

/// Grava o revisor escolhido para a run (já validado por [`guard::reviewer`]).
pub fn set_reviewer(db: &Connection, id: i64, reviewer: &str) -> Result<()> {
    db.execute(
        "UPDATE runs SET reviewer=?2 WHERE id=?1",
        params![id, reviewer],
    )?;
    Ok(())
}

/// Grava o veredito da contraprova (valor validado pela migração de `runs.proof`).
pub fn set_proof(db: &Connection, id: i64, verdict: &str) -> Result<()> {
    db.execute("UPDATE runs SET proof=?2 WHERE id=?1", params![id, verdict])?;
    Ok(())
}

/// Tarefas de runs ainda não terminais, para reconciliar o card depois de uma queda.
pub fn active_runs(db: &Connection) -> Result<Vec<(i64, i64, i64)>> {
    let mut query =
        db.prepare("SELECT id, task_id, started_at FROM runs WHERE state NOT IN (?1,?2,?3,?4,?5)")?;
    let rows = query.query_map(
        params![
            TERMINAL[0],
            TERMINAL[1],
            TERMINAL[2],
            TERMINAL[3],
            TERMINAL[4]
        ],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Runs que estavam ativas quando o app fechou não têm mais processo.
pub fn mark_interrupted(db: &Connection) -> Result<()> {
    db.execute(
        "UPDATE runs SET state=?1, finished_at=?2, summary='O Harnessy foi fechado durante a run.' \
         WHERE state NOT IN (?3,?4,?5,?6,?7)",
        params![
            INTERRUPTED, now(), TERMINAL[0], TERMINAL[1], TERMINAL[2], TERMINAL[3], TERMINAL[4]
        ],
    )?;
    Ok(())
}

/// Grava um evento (já redigido e truncado). `None` quando o log estourou o limite.
pub fn append_event(
    db: &Connection,
    run_id: i64,
    kind: &str,
    text: &str,
) -> Result<Option<RunEvent>> {
    let (used, seq): (i64, i64) = db.query_row(
        "SELECT COALESCE(SUM(LENGTH(text)), 0), COALESCE(MAX(seq), 0) + 1 FROM run_events WHERE run_id=?1",
        [run_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if used + text.len() as i64 > EVENT_LOG_LIMIT {
        return Ok(None);
    }
    db.execute(
        "INSERT INTO run_events(run_id, seq, kind, text) VALUES (?1, ?2, ?3, ?4)",
        params![run_id, seq, kind, text],
    )?;
    Ok(Some(RunEvent {
        run_id,
        seq,
        kind: kind.to_owned(),
        text: text.to_owned(),
    }))
}

pub fn events(db: &Connection, run_id: i64, after_seq: i64) -> Result<Vec<RunEvent>> {
    let mut query = db.prepare(
        "SELECT run_id, seq, kind, text FROM run_events WHERE run_id=?1 AND seq>?2 ORDER BY seq",
    )?;
    let rows = query.query_map(params![run_id, after_seq], |r| {
        Ok(RunEvent {
            run_id: r.get(0)?,
            seq: r.get(1)?,
            kind: r.get(2)?,
            text: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    fn new_run(workspace_id: i64) -> NewRun<'static> {
        NewRun {
            workspace_id,
            task_id: 1,
            title: "Ajustar texto",
            action: "implement",
            role: "implementador",
            runtime: "claude",
            model: "sonnet",
            effort: None,
            base_run_id: None,
        }
    }

    fn cycle(n: i64, usage: CycleUsage) -> CycleRecord<'static> {
        CycleRecord {
            cycle: n,
            model: "sonnet",
            session_id: Some("sessao-1"),
            resumed: false,
            usage,
            trigger: if n == 1 {
                TRIGGER_INITIAL
            } else {
                TRIGGER_CHECK_FAILED
            },
            started_at: 10,
            finished_at: 20,
            agent_ms: 1000,
        }
    }

    fn known(input: u64, cost: f64) -> CycleUsage {
        CycleUsage {
            input: Some(input),
            output: Some(5),
            cache_read: Some(1),
            cache_write: Some(2),
            cost_usd: Some(cost),
            turns: Some(3),
        }
    }

    #[test]
    fn denial_keys_group_by_command_and_are_remembered_per_workspace() {
        assert_eq!(denial_key("Bash: curl -s https://x"), "curl");
        assert_eq!(denial_key("Bash: git push origin main"), "git push");
        assert_eq!(denial_key("Bash: gh --version"), "gh");
        assert_eq!(denial_key("WebFetch"), "WebFetch");
        assert_eq!(denial_key("Edit: .env"), "Edit");

        let temp = TempDatabase::new("run-denials");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        let denied =
            |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
        record_denials(
            &db,
            1,
            &denied(&["Bash: curl a", "Bash: curl b", "Bash: git push"]),
        )
        .unwrap();
        record_denials(&db, 1, &denied(&["Bash: git push origin x", "?"])).unwrap();
        assert_eq!(top_denials(&db, 1).unwrap(), ["git push", "curl"]);
        assert!(top_denials(&db, 2).unwrap().is_empty());
    }

    #[test]
    fn two_cycles_are_summed_and_unknown_fields_stay_null() {
        let temp = TempDatabase::new("run-ledger");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();
        let run = create(&db, &new_run(1)).unwrap();
        assert_eq!(run.model.as_deref(), Some("sonnet"));

        record_cycle(&db, run.id, &cycle(1, known(100, 0.5))).unwrap();
        record_cycle(&db, run.id, &cycle(2, known(40, 0.25))).unwrap();
        add_checks_ms(&db, run.id, 300).unwrap();
        let total = get(&db, run.id).unwrap();
        assert_eq!(total.cycles, Some(2));
        assert_eq!(total.turns, Some(6));
        assert_eq!(total.input_tokens, Some(140));
        assert_eq!(total.cache_write_tokens, Some(4));
        assert_eq!(total.cost_usd, Some(0.75));
        assert_eq!(total.agent_ms, Some(2000));
        assert_eq!(total.checks_ms, Some(300));

        // Um ciclo que não informou o custo deixa o total de custo desconhecido.
        let unknown = CycleUsage {
            cost_usd: None,
            ..known(1, 0.0)
        };
        record_cycle(&db, run.id, &cycle(3, unknown)).unwrap();
        let total = get(&db, run.id).unwrap();
        assert_eq!(total.cycles, Some(3));
        assert_eq!(total.input_tokens, Some(141));
        assert_eq!(total.cost_usd, None);

        let rows: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM run_cycles WHERE run_id=?1 AND \"trigger\"=?2",
                params![run.id, TRIGGER_CHECK_FAILED],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 2);
    }

    #[test]
    fn cycle_without_usage_leaves_every_total_null() {
        let temp = TempDatabase::new("run-ledger-null");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();
        let run = create(&db, &new_run(1)).unwrap();
        record_cycle(&db, run.id, &cycle(1, CycleUsage::default())).unwrap();
        let total = get(&db, run.id).unwrap();
        assert_eq!(total.cycles, Some(1));
        assert_eq!(total.input_tokens, None);
        assert_eq!(total.turns, None);
        assert_eq!(total.cost_usd, None);
    }

    #[test]
    fn one_active_run_per_workspace_and_terminal_states_stick() {
        let temp = TempDatabase::new("run");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();

        let run = create(&db, &new_run(1)).unwrap();
        assert!(create(&db, &new_run(1)).is_err());
        assert!(set_state(&db, run.id, IMPLEMENTING, None).unwrap());
        assert!(set_state(&db, run.id, CANCELLED, Some("Cancelada.")).unwrap());
        assert!(!set_state(&db, run.id, FAILED, Some("depois")).unwrap());
        let run = get(&db, run.id).unwrap();
        assert_eq!(run.state, CANCELLED);
        assert_eq!(run.summary, "Cancelada.");
        assert!(run.finished_at.is_some());
        assert!(create(&db, &new_run(1)).is_ok());
    }

    #[test]
    fn stale_active_runs_become_interrupted() {
        let temp = TempDatabase::new("run-interrupt");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();
        let run = create(&db, &new_run(1)).unwrap();
        mark_interrupted(&db).unwrap();
        assert_eq!(get(&db, run.id).unwrap().state, INTERRUPTED);
    }

    #[test]
    fn active_runs_lists_only_non_terminal_runs() {
        let temp = TempDatabase::new("run-active");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();
        let run = create(&db, &new_run(1)).unwrap();
        let active = active_runs(&db).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!((active[0].0, active[0].1), (run.id, 1));
        mark_interrupted(&db).unwrap();
        assert!(active_runs(&db).unwrap().is_empty());
    }

    #[test]
    fn events_are_ordered_and_capped() {
        let temp = TempDatabase::new("run-events");
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute(
            "INSERT INTO workspaces(name, folder) VALUES ('w', '/tmp/w')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 't', '', '')",
            [],
        )
        .unwrap();
        let run = create(&db, &new_run(1)).unwrap();
        assert_eq!(
            append_event(&db, run.id, "log", "a").unwrap().unwrap().seq,
            1
        );
        assert_eq!(
            append_event(&db, run.id, "log", "b").unwrap().unwrap().seq,
            2
        );
        let big = "x".repeat(EVENT_LOG_LIMIT as usize);
        assert!(append_event(&db, run.id, "log", &big).unwrap().is_none());
        let after_first = events(&db, run.id, 1).unwrap();
        assert_eq!(after_first.len(), 1);
        assert_eq!(after_first[0].text, "b");
    }
}
