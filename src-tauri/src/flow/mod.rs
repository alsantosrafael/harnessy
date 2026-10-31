//! Orquestração de uma run: worktree → agente → verificações → correção → commit.
//!
//! Camada de composição entre `task`, `workspace` e `run` (os domínios não se
//! importam entre si). O pipeline depende só do contrato [`Runtime`] e não sabe
//! qual CLI executa o agente; relata progresso e ledger por [`Sink`], implementado
//! em `commands.rs` com banco e eventos. As verificações do projeto ficam em
//! [`verify`]: quem decide se a run passou é o núcleo, não a mensagem do agente.

pub mod actions;
pub mod commands;
pub mod delivery;
#[cfg(test)]
mod economico;
#[cfg(test)]
mod experiment;
mod failure;
mod likely;
mod proof;
mod read;
pub mod review;
pub mod sync;
pub mod verify;

use std::{path::PathBuf, time::Instant};

use crate::{
    db, process,
    run::{
        self,
        runtime::{Access, AgentEvent, AgentSpec, Exit, Finish, Runtime, Session},
        supervisor::Registry,
        worktree, CycleRecord,
    },
};

pub const ROLE: &str = "implementador";
#[cfg(test)]
pub const RUNTIME: &str = "claude";
const MAX_SUMMARY_BYTES: usize = 2000;

/// Destino do progresso da run (persistência e eventos para a interface).
pub trait Sink {
    fn event(&self, run_id: i64, kind: &str, text: &str);
    fn state(&self, run_id: i64, state: &str, summary: Option<&str>);
    /// Motivo objetivo de a run ter ficado presa; chamado antes de `state`.
    fn blocked(&self, run_id: i64, reason: &str);
    fn worktree(&self, run_id: i64, base_sha: &str, branch: &str, path: &str);
    /// Ledger: um ciclo do agente terminou (com ou sem uso informado).
    fn cycle(&self, run_id: i64, cycle: &CycleRecord<'_>);
    /// Tempo gasto nos checks depois de um ciclo.
    fn checks_ms(&self, run_id: i64, ms: i64);
    /// Veredito da contraprova (`proved`, `not_proved`, `no_tests`, `skipped`, `inconclusive`).
    fn proof(&self, run_id: i64, verdict: &str);
    /// Ações negadas na run, para o workspace lembrar delas no próximo prompt.
    fn denials(&self, _run_id: i64, _denied: &[String]) {}
    /// Veredito do revisor (`approve`, `request_changes`, `invalid`) e achados descartados.
    fn review(&self, _run_id: i64, _verdict: &str, _discarded: usize) {}
}

pub struct Job {
    pub run_id: i64,
    pub title: String,
    pub prompt: String,
    pub repo: PathBuf,
    pub data_dir: PathBuf,
    pub model: String,
    pub checks: Vec<verify::Check>,
    /// Tentativas de correção além da primeira implementação. Zero desliga o loop.
    pub max_corrections: u32,
    /// Contraprova ligada (Ação com `contraprova` ausente ou `true`).
    pub contraprova: bool,
    /// Sem teste que prove a mudança, pede um num ciclo de correção.
    pub require_proof: bool,
    /// Esforço de raciocínio da Ação, já validado.
    pub effort: Option<String>,
    /// Teto de turnos por ciclo (Ação, nunca acima de `guard::MAX_TURNS`).
    pub max_turns: u32,
    /// Revisor independente da run verde; `None` desliga a revisão.
    pub review: Option<review::Reviewer>,
    /// Branch de uma run anterior que serve de base (passo encadeado); `None`
    /// usa o `HEAD` do repositório.
    pub base_ref: Option<String>,
    /// JSON Schema da saída de uma Ação de leitura estruturada (`passos`).
    pub output_schema: Option<&'static str>,
}

/// Onde e como a sequência de ciclos começa.
struct Plan {
    dir: PathBuf,
    base: String,
    prompt: String,
    trigger: &'static str,
    /// Ciclos já gravados na run (0 numa run nova).
    cycle_offset: i64,
    corrections: u32,
}

/// Métricas e resultado de uma execução, usados pelo experimento e pelos testes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Todos os checks passaram ao fim da run.
    pub passed: bool,
    /// Chamadas ao agente, incluindo a primeira implementação.
    pub agent_calls: u32,
    /// Bytes de prompt enviados ao agente somados.
    pub prompt_bytes: usize,
}

pub fn execute(job: Job, registry: &Registry, sink: &dyn Sink, runtime: &dyn Runtime) -> Summary {
    let mut summary = Summary::default();
    if let Err(reason) = pipeline(&job, registry, sink, runtime, &mut summary) {
        finish(sink, job.run_id, run::FAILED, &reason, &[]);
    }
    registry.unregister(job.run_id);
    summary
}

/// Ciclo extra numa run que falhou, com outro modelo (`job.model`), pedido pelo
/// usuário. Reaproveita o worktree preservado; começa em sessão nova com a
/// tarefa, as falhas atuais e o resumo do diff, e não tem correção extra.
pub fn continue_with(
    job: Job,
    dir: PathBuf,
    base: String,
    cycle_offset: i64,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
) -> Summary {
    let mut summary = Summary::default();
    let result = (|| {
        let dirs = process::search_dirs();
        let git = process::find_binary("git", &dirs).ok_or("git não encontrado.")?;
        if !dir.is_dir() {
            return Err("A pasta de trabalho da run não existe mais.".to_owned());
        }
        sink.event(job.run_id, "status", "Conferindo o estado atual…");
        let reports = verify::run_all(&job.checks, &dirs, &dir)?;
        let failing = verify::failing(&reports);
        let stat = worktree::diff_stat(&git, &dirs, &dir, &base).unwrap_or_default();
        let mut prompt = job.prompt.clone();
        prompt.push_str("\n\n## Estado atual\nUma tentativa anterior com outro modelo não terminou com as verificações verdes. As alterações dela estão no diretório; continue a partir delas.\n");
        if !stat.trim().is_empty() {
            prompt.push_str(&format!("\nArquivos alterados:\n```\n{stat}\n```\n"));
        }
        if !failing.is_empty() {
            prompt.push('\n');
            prompt.push_str(&verify::correction_prompt(&job.title, &failing));
        }
        let plan = Plan {
            dir,
            base,
            prompt,
            trigger: run::TRIGGER_BIGGER_MODEL,
            cycle_offset,
            corrections: 0,
        };
        work(
            &job,
            &plan,
            &git,
            &dirs,
            registry,
            sink,
            runtime,
            &mut summary,
        )
    })();
    if let Err(reason) = result {
        finish(sink, job.run_id, run::FAILED, &reason, &[]);
    }
    registry.unregister(job.run_id);
    summary
}

fn pipeline(
    job: &Job,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
    summary: &mut Summary,
) -> Result<(), String> {
    let id = job.run_id;
    let dirs = process::search_dirs();
    let git = process::find_binary("git", &dirs).ok_or("git não encontrado.")?;
    let bun = process::find_binary("bun", &dirs);

    sink.event(id, "status", "Preparando worktree…");
    let base = match &job.base_ref {
        Some(branch) => {
            sink.event(id, "status", &format!("Continuando da branch {branch}…"));
            worktree::branch_sha(&git, &dirs, &job.repo, branch)?
        }
        None => worktree::head_sha(&git, &dirs, &job.repo)?,
    };
    let branch = worktree::branch_name(id, &job.title);
    let dir = job.data_dir.join("worktrees").join(format!("run-{id}"));
    std::fs::create_dir_all(job.data_dir.join("worktrees")).map_err(|e| e.to_string())?;
    worktree::create(&git, &dirs, &job.repo, &dir, &branch, &base)?;
    sink.worktree(id, &base, &branch, &dir.to_string_lossy());

    sink.event(id, "status", "Instalando dependências…");
    worktree::install_deps(bun.as_deref(), &dirs, &dir)?;
    if registry.is_cancelled(id) {
        finish(sink, id, run::CANCELLED, "Run cancelada.", &[]);
        return Ok(());
    }
    let plan = Plan {
        dir,
        base,
        prompt: job.prompt.clone(),
        trigger: run::TRIGGER_INITIAL,
        cycle_offset: 0,
        corrections: job.max_corrections,
    };
    work(job, &plan, &git, &dirs, registry, sink, runtime, summary)
}

/// Resultado de um ciclo do agente, já com o recuo de retomada aplicado.
struct Cycle {
    exit: Exit,
    session_id: Option<String>,
}

/// Agente → verificações → correção, até verde ou fim das correções; depois
/// commit e contraprova. Correções retomam a sessão quando o runtime sabe.
#[allow(clippy::too_many_arguments)]
fn work(
    job: &Job,
    plan: &Plan,
    git: &std::path::Path,
    dirs: &[PathBuf],
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
    summary: &mut Summary,
) -> Result<(), String> {
    let id = job.run_id;
    let dir = &plan.dir;
    // Ações que o agente negou, somadas entre as tentativas: sinal de bloqueio sem custo.
    let mut denied: Vec<String> = Vec::new();
    let mut prompt = plan.prompt.clone();
    let mut resume: Option<String> = None;
    let mut trigger = plan.trigger;
    // Ciclos do revisor também numeram `run_cycles`, mas não gastam correção.
    let mut review_cycles: i64 = 0;
    let mut reviewed = false;
    // `exigir_prova` pede no máximo um ciclo extra por run.
    let mut proved_once = false;

    loop {
        summary.agent_calls += 1;
        let number = plan.cycle_offset + i64::from(summary.agent_calls) + review_cycles;
        sink.state(id, run::IMPLEMENTING, None);
        sink.event(
            id,
            "status",
            &format!(
                "Agente em execução (ciclo {number}, modelo {}{})…",
                job.model,
                if resume.is_some() {
                    ", sessão retomada"
                } else {
                    ""
                }
            ),
        );
        let started_at = db::now();
        let started = Instant::now();
        let mut finish_event: Option<Finish> = None;
        let mut resumed = resume.is_some();
        let mut cycle = run_cycle(
            job,
            dir,
            &prompt,
            resume.as_deref(),
            registry,
            sink,
            runtime,
            &mut denied,
            &mut finish_event,
        );
        // Recuo: retomada recusada antes de qualquer evento vira sessão nova com o
        // contexto completo, sem contar como ciclo.
        if resumed && finish_event.is_none() && cycle.session_id.is_none() {
            if let Exit::Failed(_) = cycle.exit {
                sink.event(id, "status", "Sessão não retomada; contexto reenviado.");
                prompt = format!("{}\n\n{prompt}", job.prompt);
                resumed = false;
                cycle = run_cycle(
                    job,
                    dir,
                    &prompt,
                    None,
                    registry,
                    sink,
                    runtime,
                    &mut denied,
                    &mut finish_event,
                );
            }
        }
        summary.prompt_bytes += prompt.len();
        let usage = finish_event
            .as_ref()
            .map(|done| done.usage)
            .unwrap_or_default();
        let session_id = finish_event
            .as_ref()
            .and_then(|done| done.session_id.clone())
            .or(cycle.session_id);
        sink.cycle(
            id,
            &CycleRecord {
                cycle: number,
                model: &job.model,
                session_id: session_id.as_deref(),
                resumed,
                usage,
                trigger,
                started_at,
                finished_at: db::now(),
                agent_ms: started.elapsed().as_millis() as i64,
            },
        );
        let agent_summary = match cycle.exit {
            Exit::Success(text) => text,
            Exit::Failed(reason) => {
                finish(sink, id, run::FAILED, &clip(&reason), &denied);
                return Ok(());
            }
            Exit::TimedOut => {
                finish(
                    sink,
                    id,
                    run::FAILED,
                    "Tempo limite da run excedido.",
                    &denied,
                );
                return Ok(());
            }
            Exit::Cancelled => {
                finish(sink, id, run::CANCELLED, "Run cancelada.", &[]);
                return Ok(());
            }
        };

        sink.event(id, "status", "Rodando verificações…");
        let checks_started = Instant::now();
        let reports = verify::run_all(&job.checks, dirs, dir)?;
        sink.checks_ms(id, checks_started.elapsed().as_millis() as i64);
        let failing = verify::failing(&reports);
        if !failing.is_empty() {
            for report in &failing {
                sink.event(id, "status", &format!("Falhou: {}", report.name));
            }
            let failures = verify::failures(&failing);
            if let Ok(json) = serde_json::to_string(&failures) {
                sink.event(id, "failures", &json);
            }
            if summary.agent_calls > plan.corrections {
                let reason = "As verificações ainda falham depois das tentativas de correção.";
                finish(sink, id, run::FAILED, reason, &denied);
                return Ok(());
            }
            let correction = verify::correction_prompt(&job.title, &failing);
            (resume, prompt) = next_prompt(job, runtime, session_id, correction);
            trigger = run::TRIGGER_CHECK_FAILED;
            continue;
        }
        summary.passed = true;

        let message = format!("harnessy: {}", job.title);
        if !worktree::commit_all(git, dirs, dir, &plan.base, &message)? {
            // O resumo do agente explica por que nada mudou: é o que o card mostra.
            let why = clip(&agent_summary);
            let why = if why.trim().is_empty() {
                "O agente não alterou nada."
            } else {
                &why
            };
            finish(sink, id, run::NO_CHANGES, why, &denied);
            let _ = worktree::remove(git, dirs, &job.repo, dir);
            return Ok(());
        }
        // Contraprova depois do commit: só aviso, o estado final não muda.
        let proof = proof::run(git, dirs, dir, &plan.base, &job.checks, job.contraprova);
        sink.proof(id, proof.verdict.as_str());
        sink.event(id, "status", &proof.message());
        sink.event(id, "proof", &proof.event_json());

        if job.require_proof
            && !proved_once
            && matches!(
                proof.verdict,
                proof::Verdict::NotProved | proof::Verdict::NoTests
            )
        {
            proved_once = true;
            if summary.agent_calls <= plan.corrections {
                let head = format!(
                    "As verificações passaram, mas nenhum teste prova a mudança da tarefa \"{}\".\n\n",
                    job.title
                );
                let failure = failure::Failure {
                    check: "contraprova".into(),
                    file: None,
                    line: None,
                    test_name: None,
                    expected: Some("um teste que falha sem a mudança e passa com ela".into()),
                    actual: Some(proof.message()),
                    excerpt: "Escreva ou ajuste um teste que exercite o comportamento pedido. Ele precisa falhar sem a sua mudança de código.".into(),
                };
                let correction = verify::failure_prompt(head, &[failure]);
                (resume, prompt) = next_prompt(job, runtime, session_id, correction);
                trigger = run::TRIGGER_PROOF;
                summary.passed = false;
                continue;
            }
        }

        if !reviewed && job.review.is_none() {
            reviewed = true;
            sink.event(
                id,
                "status",
                "Revisão não rodou: está desligada para esta run (escolha de revisão \"Nunca\" ou Ação `revisar` em modo manual).",
            );
        }
        if let (false, Some(reviewer)) = (reviewed, job.review.as_ref()) {
            reviewed = true;
            let at = number + 1;
            if let Some(outcome) = review_cycle(
                job, reviewer, plan, git, dirs, &proof, at, registry, sink, runtime,
            ) {
                review_cycles += 1;
                if outcome.verdict == "request_changes" {
                    if summary.agent_calls <= plan.corrections {
                        let head = format!(
                            "A revisão independente apontou defeitos na sua alteração da tarefa \"{}\".\n\n",
                            job.title
                        );
                        let correction = verify::failure_prompt(head, &outcome.failures());
                        (resume, prompt) = next_prompt(job, runtime, session_id, correction);
                        trigger = run::TRIGGER_REVIEW;
                        summary.passed = false;
                        continue;
                    }
                    sink.event(
                        id,
                        "status",
                        "A revisão pediu mudanças, mas não há ciclos de correção restantes; revise os achados.",
                    );
                }
            }
        }

        finish(sink, id, run::COMPLETED, &clip(&agent_summary), &denied);
        if !proof.keep_worktree {
            let _ = worktree::remove(git, dirs, &job.repo, dir);
        }
        return Ok(());
    }
}

/// Próximo prompt de correção: só a falha se a sessão for retomada; senão a
/// tarefa completa seguida da falha.
fn next_prompt(
    job: &Job,
    runtime: &dyn Runtime,
    session_id: Option<String>,
    correction: String,
) -> (Option<String>, String) {
    let resume = session_id.filter(|_| runtime.resumes());
    let prompt = if resume.is_some() {
        correction
    } else {
        format!("{}\n\n{correction}", job.prompt)
    };
    (resume, prompt)
}

/// Revisão independente da run verde, quando devida. Grava o ciclo do revisor
/// no ledger e o veredito na run; `None` quando não roda ou não consegue rodar.
#[allow(clippy::too_many_arguments)]
fn review_cycle(
    job: &Job,
    reviewer: &review::Reviewer,
    plan: &Plan,
    git: &std::path::Path,
    dirs: &[PathBuf],
    proof: &proof::Proof,
    number: i64,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
) -> Option<review::Outcome> {
    let id = job.run_id;
    let diffs = match review::diffs(git, dirs, &plan.dir, &plan.base) {
        Ok(diffs) => diffs,
        Err(_) => {
            sink.event(
                id,
                "status",
                "Revisão não rodou: não foi possível ler o diff.",
            );
            return None;
        }
    };
    if !review::is_due(reviewer.action.quando, proof.verdict, diffs.size) {
        sink.event(id, "status", &review::skipped_message(proof.verdict));
        return None;
    }
    let checks: String = job
        .checks
        .iter()
        .map(|check| format!("- {}: passou\n", check.name))
        .collect();
    Some(review_diff(
        id,
        reviewer,
        &diffs,
        &plan.dir,
        &checks,
        &proof.message(),
        number,
        registry,
        sink,
        runtime,
    ))
}

/// Roda o revisor sobre um diff já calculado e grava ciclo, veredito e eventos.
#[allow(clippy::too_many_arguments)]
fn review_diff(
    id: i64,
    reviewer: &review::Reviewer,
    diffs: &review::Diffs,
    dir: &std::path::Path,
    checks: &str,
    proof_message: &str,
    number: i64,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
) -> review::Outcome {
    sink.event(
        id,
        "status",
        &format!("Revisão independente ({})…", reviewer.model),
    );
    let prompt = review::prompt_for(reviewer, &diffs.unified, checks, proof_message);
    let mut spec = AgentSpec::new(
        dir,
        &prompt,
        &reviewer.model,
        actions::max_turns(&reviewer.action),
        Access::Read,
    );
    spec.effort = reviewer.action.esforco.as_deref();
    spec.json_schema = Some(review::SCHEMA);
    let session = Session {
        run_id: id,
        registry,
    };
    let started_at = db::now();
    let started = Instant::now();
    let mut finish_event: Option<Finish> = None;
    let mut session_id: Option<String> = None;
    let review_runtime: &dyn Runtime = match &reviewer.runtime {
        Some(own) => own.as_ref(),
        None => runtime,
    };
    let exit = review_runtime.start(&spec, &session, &mut |event| {
        match &event {
            AgentEvent::Started { session_id: sid } => session_id = Some(sid.clone()),
            AgentEvent::Finished(done) => finish_event = Some(done.clone()),
            _ => {}
        }
        if let Some((kind, text)) = event.log_line() {
            sink.event(id, kind, &text);
        }
    });
    sink.cycle(
        id,
        &CycleRecord {
            cycle: number,
            model: &reviewer.model,
            session_id: finish_event
                .as_ref()
                .and_then(|done| done.session_id.as_deref())
                .or(session_id.as_deref()),
            resumed: false,
            usage: finish_event
                .as_ref()
                .map(|done| done.usage)
                .unwrap_or_default(),
            trigger: run::TRIGGER_REVIEW,
            started_at,
            finished_at: db::now(),
            agent_ms: started.elapsed().as_millis() as i64,
        },
    );
    let text = match &exit {
        Exit::Success(text) => text.as_str(),
        _ => "",
    };
    let outcome = review::evaluate(
        finish_event
            .as_ref()
            .and_then(|done| done.structured.as_deref()),
        text,
        &diffs.changed,
    );
    sink.review(id, outcome.verdict, outcome.discarded);
    sink.event(id, "status", &outcome.message());
    sink.event(id, "review", &outcome.event_json(&reviewer.model));
    outcome
}

/// Revisão pedida à mão sobre uma run concluída: o diff vem da branch dela
/// (o worktree já foi removido), no repositório do workspace. O veredito e o
/// ciclo do revisor entram na run como numa revisão automática; o estado dela
/// e o card não mudam.
#[allow(clippy::too_many_arguments)]
pub fn review_existing(
    run_id: i64,
    reviewer: &review::Reviewer,
    repo: &std::path::Path,
    base: &str,
    branch: &str,
    proof: &str,
    number: i64,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
) {
    let dirs = process::search_dirs();
    let diffs = process::find_binary("git", &dirs)
        .ok_or_else(|| "git não encontrado.".to_owned())
        .and_then(|git| review::diffs_between(&git, &dirs, repo, base, branch));
    match diffs {
        Err(reason) => sink.event(
            run_id,
            "status",
            &format!("Revisão não rodou: {reason} Confira se a branch ainda existe."),
        ),
        Ok(diffs) if diffs.size == 0 => sink.event(
            run_id,
            "status",
            "Revisão não rodou: a branch não tem alterações.",
        ),
        Ok(diffs) => {
            let proof = format!("Contraprova registrada na run: {proof}.");
            review_diff(
                run_id,
                reviewer,
                &diffs,
                repo,
                "- verificações da run: passaram\n",
                &proof,
                number,
                registry,
                sink,
                runtime,
            );
        }
    }
    registry.unregister(run_id);
}

/// Um ciclo do agente: eventos para o log, negações e fim reportado.
#[allow(clippy::too_many_arguments)]
fn run_cycle(
    job: &Job,
    dir: &std::path::Path,
    prompt: &str,
    resume: Option<&str>,
    registry: &Registry,
    sink: &dyn Sink,
    runtime: &dyn Runtime,
    denied: &mut Vec<String>,
    finish_event: &mut Option<Finish>,
) -> Cycle {
    let id = job.run_id;
    let mut spec = AgentSpec::new(dir, prompt, &job.model, job.max_turns, Access::Write);
    spec.effort = job.effort.as_deref();
    spec.resume = resume;
    let session = Session {
        run_id: id,
        registry,
    };
    let mut session_id: Option<String> = None;
    let exit = runtime.start(&spec, &session, &mut |event| {
        match &event {
            AgentEvent::Denied(text) if !denied.contains(text) => denied.push(text.clone()),
            AgentEvent::Started { session_id: sid } => session_id = Some(sid.clone()),
            AgentEvent::Finished(done) => *finish_event = Some(done.clone()),
            _ => {}
        }
        if let Some((kind, text)) = event.log_line() {
            sink.event(id, kind, &text);
        }
    });
    Cycle { exit, session_id }
}

fn finish(sink: &dyn Sink, run_id: i64, state: &str, summary: &str, denied: &[String]) {
    if !denied.is_empty() {
        sink.denials(run_id, denied);
    }
    let reason = sync::blocker(state, summary, denied);
    if !reason.is_empty() {
        sink.blocked(run_id, &reason);
    }
    sink.state(run_id, state, Some(summary));
}

fn clip(text: &str) -> String {
    let mut end = text.len().min(MAX_SUMMARY_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, path::Path, process::Command};

    use super::*;
    use crate::{
        db::test_support::TempDatabase,
        run::{runtime::CycleUsage, NewRun},
    };

    /// Agente falso: a primeira tentativa não cria o arquivo que o check exige;
    /// a segunda cria. `usage` define o que cada ciclo informa.
    struct FakeAgent {
        calls: Cell<usize>,
        usage: [CycleUsage; 2],
    }

    impl Runtime for FakeAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            session: &Session<'_>,
            on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            let call = self.calls.get();
            self.calls.set(call + 1);
            if call == 1 {
                std::fs::write(spec.dir.join("ok.txt"), "feito").unwrap();
            }
            on_event(AgentEvent::Started {
                session_id: format!("sessao-{}", session.run_id),
            });
            on_event(AgentEvent::Text("Alterei o arquivo.".into()));
            on_event(AgentEvent::Finished(Finish {
                ok: true,
                subtype: "success".into(),
                summary: "Pronto.".into(),
                session_id: None,
                usage: self.usage[call],
                denials: 0,
                structured: None,
            }));
            Exit::Success("Pronto.".into())
        }
    }

    /// Sink que grava o ledger no banco de teste, como o `AppSink` faz.
    struct DbSink<'a> {
        store: &'a crate::db::Database,
    }

    impl Sink for DbSink<'_> {
        fn event(&self, _: i64, _: &str, _: &str) {}
        fn state(&self, run_id: i64, state: &str, summary: Option<&str>) {
            let db = self.store.connection().unwrap();
            run::set_state(&db, run_id, state, summary).unwrap();
        }
        fn blocked(&self, run_id: i64, reason: &str) {
            let db = self.store.connection().unwrap();
            run::set_blocked(&db, run_id, reason).unwrap();
        }
        fn worktree(&self, run_id: i64, base_sha: &str, branch: &str, path: &str) {
            let db = self.store.connection().unwrap();
            run::set_worktree(&db, run_id, base_sha, branch, path).unwrap();
        }
        fn cycle(&self, run_id: i64, cycle: &CycleRecord<'_>) {
            let db = self.store.connection().unwrap();
            run::record_cycle(&db, run_id, cycle).unwrap();
        }
        fn checks_ms(&self, run_id: i64, ms: i64) {
            let db = self.store.connection().unwrap();
            run::add_checks_ms(&db, run_id, ms).unwrap();
        }
        fn proof(&self, run_id: i64, verdict: &str) {
            let db = self.store.connection().unwrap();
            run::set_proof(&db, run_id, verdict).unwrap();
        }
        fn review(&self, run_id: i64, verdict: &str, discarded: usize) {
            let db = self.store.connection().unwrap();
            run::set_review(&db, run_id, verdict, discarded).unwrap();
        }
        fn denials(&self, run_id: i64, denied: &[String]) {
            let db = self.store.connection().unwrap();
            let workspace_id = run::get(&db, run_id).unwrap().workspace_id;
            run::record_denials(&db, workspace_id, denied).unwrap();
        }
    }

    /// Chamada recebida pelo agente roteirizado.
    #[derive(Debug, Clone)]
    struct Call {
        prompt: String,
        resume: Option<String>,
        model: String,
    }

    /// Agente roteirizado: cria `ok.txt` a partir da chamada `fix_at` (contando
    /// só as que respondem), nega um `curl` na primeira e pode recusar retomada.
    struct CycleAgent {
        resumes: bool,
        refuse_resume: bool,
        fix_at: usize,
        calls: std::cell::RefCell<Vec<Call>>,
        answered: Cell<usize>,
    }

    impl CycleAgent {
        fn new(resumes: bool, refuse_resume: bool, fix_at: usize) -> Self {
            Self {
                resumes,
                refuse_resume,
                fix_at,
                calls: Default::default(),
                answered: Cell::new(0),
            }
        }
    }

    impl Runtime for CycleAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            session: &Session<'_>,
            on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            self.calls.borrow_mut().push(Call {
                prompt: spec.prompt.to_owned(),
                resume: spec.resume.map(str::to_owned),
                model: spec.model.to_owned(),
            });
            if spec.resume.is_some() && self.refuse_resume {
                return Exit::Failed("No conversation found".into());
            }
            let n = self.answered.get();
            self.answered.set(n + 1);
            if n >= self.fix_at {
                std::fs::write(spec.dir.join("ok.txt"), "feito").unwrap();
            }
            on_event(AgentEvent::Started {
                session_id: format!("sessao-{}-{n}", session.run_id),
            });
            if n == 0 {
                on_event(AgentEvent::Denied("Bash: curl -s https://exemplo".into()));
            }
            Exit::Success("Pronto.".into())
        }

        fn resumes(&self) -> bool {
            self.resumes
        }
    }

    /// Run com o check `test -f ok.txt`; devolve banco, id e a pasta temporária.
    fn scripted_run(
        label: &str,
        agent: &CycleAgent,
        max_corrections: u32,
    ) -> (TempDatabase, i64, TempRepo) {
        let repo = TempRepo::new(label);
        let temp = TempDatabase::new(&format!("flow-{label}"));
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Criar arquivo', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Criar arquivo",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "haiku",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let registry = Registry::default();
        registry.register(run_id);
        execute(
            scripted_job(run_id, &repo, "haiku", max_corrections),
            &registry,
            &DbSink { store: &store },
            agent,
        );
        (temp, run_id, repo)
    }

    fn scripted_job(run_id: i64, repo: &TempRepo, model: &str, max_corrections: u32) -> Job {
        Job {
            run_id,
            title: "Criar arquivo".into(),
            prompt: "TAREFA: crie ok.txt.".into(),
            repo: repo.repo.clone(),
            data_dir: repo.data.clone(),
            model: model.into(),
            checks: vec![verify::Check {
                name: "Arquivo".into(),
                program: "sh".into(),
                args: vec!["-c".into(), "test -f ok.txt".into()],
                kind: verify::CheckKind::Types,
            }],
            max_corrections,
            contraprova: false,
            require_proof: false,
            effort: Some("high".into()),
            max_turns: 7,
            review: None,
            base_ref: None,
            output_schema: None,
        }
    }

    fn resumed_flags(db: &rusqlite::Connection, run_id: i64) -> Vec<(i64, bool, String)> {
        let mut query = db
            .prepare(
                "SELECT cycle, resumed, \"trigger\" FROM run_cycles WHERE run_id=?1 ORDER BY cycle",
            )
            .unwrap();
        query
            .query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Implementador que escreve `ok.txt` a cada chamada e revisor (chamada com
    /// `json_schema`) que devolve `review_json`.
    struct ReviewedAgent {
        review_json: String,
        calls: std::cell::RefCell<Vec<Call>>,
        implementer_calls: Cell<usize>,
    }

    impl Runtime for ReviewedAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            session: &Session<'_>,
            on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            self.calls.borrow_mut().push(Call {
                prompt: spec.prompt.to_owned(),
                resume: spec.resume.map(str::to_owned),
                model: spec.model.to_owned(),
            });
            let structured = if spec.json_schema.is_some() {
                assert_eq!(spec.access, Access::Read);
                Some(self.review_json.clone())
            } else {
                let n = self.implementer_calls.get() + 1;
                self.implementer_calls.set(n);
                std::fs::write(spec.dir.join("ok.txt"), format!("versão {n}\n")).unwrap();
                None
            };
            on_event(AgentEvent::Finished(Finish {
                ok: true,
                subtype: "success".into(),
                summary: "Pronto.".into(),
                session_id: Some(format!("sessao-{}-{}", session.run_id, spec.model)),
                usage: CycleUsage::default(),
                denials: 0,
                structured,
            }));
            Exit::Success("Pronto.".into())
        }

        fn resumes(&self) -> bool {
            true
        }
    }

    fn reviewed_run(label: &str, review_json: &str) -> (TempDatabase, i64, ReviewedAgent) {
        reviewed_run_with(label, review_json, None)
    }

    fn reviewed_run_with(
        label: &str,
        review_json: &str,
        external: Option<Box<dyn Runtime + Send>>,
    ) -> (TempDatabase, i64, ReviewedAgent) {
        let (_repo, temp, run_id, agent) = reviewed_run_kept(label, review_json, external);
        (temp, run_id, agent)
    }

    /// Como [`reviewed_run_with`], mas devolve o repositório para o teste continuar usando a branch.
    fn reviewed_run_kept(
        label: &str,
        review_json: &str,
        external: Option<Box<dyn Runtime + Send>>,
    ) -> (TempRepo, TempDatabase, i64, ReviewedAgent) {
        let repo = TempRepo::new(label);
        let temp = TempDatabase::new(&format!("flow-{label}"));
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Criar arquivo', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Criar arquivo",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "haiku",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let builtin = include_str!("../action/embutidas/revisar.toml")
            .replace("quando = \"risco\"", "quando = \"sempre\"");
        let action = crate::action::parse(&builtin).unwrap();
        let mut job = scripted_job(run_id, &repo, "haiku", 3);
        job.review = Some(review::Reviewer {
            model: review::Reviewer::model_for(&action, "haiku"),
            action,
            tarefa: "Criar arquivo".into(),
            descricao: String::new(),
            criterios: "ok.txt existe".into(),
            harness: String::new(),
            runtime: external,
        });
        let agent = ReviewedAgent {
            review_json: review_json.into(),
            calls: Default::default(),
            implementer_calls: Cell::new(0),
        };
        let registry = Registry::default();
        registry.register(run_id);
        let summary = execute(job, &registry, &DbSink { store: &store }, &agent);
        assert!(summary.passed);
        drop(store);
        (repo, temp, run_id, agent)
    }

    fn cycle_rows(db: &rusqlite::Connection, run_id: i64) -> Vec<(i64, String, String)> {
        let mut query = db
            .prepare(
                "SELECT cycle, model, \"trigger\" FROM run_cycles WHERE run_id=?1 ORDER BY cycle",
            )
            .unwrap();
        query
            .query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Revisor com runtime próprio (o Codex): conta as chamadas e devolve a revisão.
    struct SeparateReviewer(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Runtime for SeparateReviewer {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            _session: &Session<'_>,
            on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            assert_eq!(spec.access, Access::Read);
            self.0.lock().unwrap().push(spec.prompt.to_owned());
            on_event(AgentEvent::Finished(Finish {
                ok: true,
                subtype: "success".into(),
                summary: "ok".into(),
                session_id: None,
                usage: CycleUsage::default(),
                denials: 0,
                structured: Some(r#"{"veredito":"approve","achados":[]}"#.into()),
            }));
            Exit::Success("ok".into())
        }
    }

    #[test]
    fn a_reviewer_with_its_own_runtime_reviews_instead_of_the_implementer_runtime() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (temp, run_id, agent) = reviewed_run_with(
            "review-external",
            r#"{"veredito":"request_changes","achados":[]}"#,
            Some(Box::new(SeparateReviewer(seen.clone()))),
        );
        assert_eq!(
            agent.calls.borrow().len(),
            1,
            "o implementador não é chamado para revisar"
        );
        let prompts = seen.lock().unwrap();
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("```diff"));
        let store = temp.open();
        let db = store.connection().unwrap();
        assert_eq!(
            run::get(&db, run_id).unwrap().review.as_deref(),
            Some("approve")
        );
    }

    #[test]
    fn a_finished_run_can_be_reviewed_by_hand_from_its_branch() {
        let (_repo, temp, run_id, _) = reviewed_run_kept(
            "review-manual",
            r#"{"veredito":"approve","achados":[]}"#,
            None,
        );
        let store = temp.open();
        let (run, folder) = {
            let db = store.connection().unwrap();
            let run = run::get(&db, run_id).unwrap();
            let folder: String = db
                .query_row("SELECT folder FROM workspaces WHERE id=1", [], |r| r.get(0))
                .unwrap();
            (run, folder)
        };
        let action =
            crate::action::parse(include_str!("../action/embutidas/revisar.toml")).unwrap();
        let reviewer = review::Reviewer {
            model: "opus".into(),
            action,
            tarefa: "Criar arquivo".into(),
            descricao: String::new(),
            criterios: "ok.txt existe".into(),
            harness: String::new(),
            runtime: None,
        };
        let agent = ReviewedAgent {
            review_json: r#"{"veredito":"request_changes","achados":[{"arquivo":"ok.txt","linha":1,"severidade":"bloqueia","texto":"Falta algo."}]}"#.into(),
            calls: Default::default(),
            implementer_calls: Cell::new(0),
        };
        let registry = Registry::default();
        registry.register(run_id);
        review_existing(
            run_id,
            &reviewer,
            std::path::Path::new(&folder),
            run.base_sha.as_deref().unwrap(),
            run.branch.as_deref().unwrap(),
            "proved",
            9,
            &registry,
            &DbSink { store: &store },
            &agent,
        );
        let db = store.connection().unwrap();
        let after = run::get(&db, run_id).unwrap();
        assert_eq!(after.review.as_deref(), Some("request_changes"));
        assert_eq!(after.state, run::COMPLETED, "a revisão não muda a run");
        assert_eq!(run::last_cycle(&db, run_id).unwrap(), 9);
        assert_eq!(agent.calls.borrow().len(), 1);
        assert!(agent.calls.borrow()[0].prompt.contains("```diff"));
    }

    #[test]
    fn review_request_changes_becomes_a_correction_cycle_once() {
        let review = r#"{"veredito":"request_changes","achados":[{"arquivo":"ok.txt","linha":1,"severidade":"bloqueia","texto":"Falta o acento esperado."}]}"#;
        let (temp, run_id, agent) = reviewed_run("review-fix", review);
        let calls = agent.calls.borrow();
        assert_eq!(
            calls.len(),
            3,
            "implementa, revisa, corrige; sem segunda revisão"
        );
        let reviewer = &calls[1];
        assert_eq!(reviewer.model, "sonnet");
        assert!(reviewer.prompt.contains("```diff"));
        assert!(reviewer.prompt.contains("+versão 1"));
        assert!(
            !reviewer.prompt.contains("TAREFA: crie"),
            "revisor não vê o prompt do implementador"
        );
        assert_eq!(reviewer.resume, None);
        let fix = &calls[2];
        assert_eq!(fix.model, "haiku");
        assert_eq!(
            fix.resume.as_deref(),
            Some(format!("sessao-{run_id}-haiku").as_str())
        );
        assert!(fix.prompt.contains("revisão independente"));
        assert!(fix.prompt.contains("Arquivo: ok.txt:1"));

        let store = temp.open();
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.review.as_deref(), Some("request_changes"));
        assert_eq!(
            cycle_rows(&db, run_id),
            vec![
                (1, "haiku".into(), "initial".into()),
                (2, "sonnet".into(), "review".into()),
                (3, "haiku".into(), "review".into()),
            ]
        );
    }

    #[test]
    fn review_findings_outside_the_diff_are_discarded_and_the_run_completes() {
        let review = r#"{"veredito":"request_changes","achados":[{"arquivo":"src/inventado.rs","linha":3,"severidade":"bloqueia","texto":"x"}]}"#;
        let (temp, run_id, agent) = reviewed_run("review-discard", review);
        assert_eq!(agent.calls.borrow().len(), 2);
        let store = temp.open();
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.review.as_deref(), Some("approve"));
        assert_eq!(run.review_discarded, Some(1));
        assert_eq!(run.cycles, Some(2));
    }

    /// Agente que exige ver o arquivo da run anterior e cria `ok.txt`.
    struct ChainedAgent;

    impl Runtime for ChainedAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            _session: &Session<'_>,
            _on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            if !spec.dir.join("passo1.txt").exists() {
                return Exit::Failed("não partiu da branch do passo anterior".into());
            }
            std::fs::write(spec.dir.join("ok.txt"), "feito").unwrap();
            Exit::Success("Pronto.".into())
        }
    }

    #[test]
    fn chained_step_starts_from_the_previous_run_branch() {
        let agent = ChainedAgent;
        let repo = TempRepo::new("chained");
        let args = ["checkout", "-q", "-b", "harnessy/run-1-passo-1"];
        assert!(Command::new("git")
            .args(args)
            .current_dir(&repo.repo)
            .status()
            .unwrap()
            .success());
        std::fs::write(repo.repo.join("passo1.txt"), "feito no passo 1").unwrap();
        for args in [
            vec!["add", "-A"],
            vec!["commit", "-q", "-m", "passo 1"],
            vec!["checkout", "-q", "-"],
        ] {
            assert!(Command::new("git")
                .args(&args)
                .current_dir(&repo.repo)
                .status()
                .unwrap()
                .success());
        }
        assert!(
            !repo.repo.join("passo1.txt").exists(),
            "repositório voltou ao ramo base"
        );

        let temp = TempDatabase::new("flow-chained");
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Passo 2', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Passo 2",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "haiku",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let mut job = scripted_job(run_id, &repo, "haiku", 0);
        job.base_ref = Some("harnessy/run-1-passo-1".into());
        let registry = Registry::default();
        registry.register(run_id);
        let summary = execute(job, &registry, &DbSink { store: &store }, &agent);
        assert!(summary.passed);
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        let branch = run.branch.unwrap();
        let shown = Command::new("git")
            .args(["show", &format!("{branch}:passo1.txt")])
            .current_dir(&repo.repo)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&shown.stdout), "feito no passo 1");
        let base_of_run = Command::new("git")
            .args(["rev-parse", "harnessy/run-1-passo-1"])
            .current_dir(&repo.repo)
            .output()
            .unwrap();
        assert_eq!(
            run.base_sha.unwrap(),
            String::from_utf8_lossy(&base_of_run.stdout).trim()
        );
    }

    #[test]
    fn required_proof_asks_once_for_a_test_when_none_was_changed() {
        let agent = CycleAgent::new(true, false, 0);
        let repo = TempRepo::new("require-proof");
        let temp = TempDatabase::new("flow-require-proof");
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Criar arquivo', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Criar arquivo",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "haiku",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let mut job = scripted_job(run_id, &repo, "haiku", 3);
        job.checks[0].kind = verify::CheckKind::Tests;
        job.contraprova = true;
        job.require_proof = true;
        let registry = Registry::default();
        registry.register(run_id);
        let summary = execute(job, &registry, &DbSink { store: &store }, &agent);
        assert!(summary.passed);
        let calls = agent.calls.borrow();
        assert_eq!(calls.len(), 2, "um ciclo extra pedindo teste, sem repetir");
        assert!(calls[1].prompt.contains("nenhum teste prova a mudança"));
        let db = store.connection().unwrap();
        assert_eq!(run::get(&db, run_id).unwrap().state, run::COMPLETED);
        assert_eq!(
            cycle_rows(&db, run_id),
            vec![
                (1, "haiku".into(), "initial".into()),
                (2, "haiku".into(), "proof".into()),
            ]
        );
    }

    #[test]
    fn correction_resumes_the_session_and_sends_only_the_failure() {
        let agent = CycleAgent::new(true, false, 1);
        let (temp, run_id, _repo) = scripted_run("resume", &agent, 3);
        let calls = agent.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].resume, None);
        assert_eq!(
            calls[1].resume.as_deref(),
            Some(format!("sessao-{run_id}-0").as_str())
        );
        assert!(!calls[1].prompt.contains("TAREFA"), "{}", calls[1].prompt);
        assert!(calls[1].prompt.contains("Falha 1 de 1"));

        let store = temp.open();
        let db = store.connection().unwrap();
        assert_eq!(run::get(&db, run_id).unwrap().state, run::COMPLETED);
        assert_eq!(
            resumed_flags(&db, run_id),
            vec![
                (1, false, "initial".into()),
                (2, true, "check_failed".into())
            ]
        );
        assert_eq!(run::top_denials(&db, 1).unwrap(), ["curl"]);
    }

    #[test]
    fn refused_resume_falls_back_with_full_context_without_counting_a_cycle() {
        let agent = CycleAgent::new(true, true, 1);
        let (temp, run_id, _repo) = scripted_run("fallback", &agent, 3);
        let calls = agent.calls.borrow();
        assert_eq!(calls.len(), 3);
        assert!(calls[1].resume.is_some());
        assert_eq!(calls[2].resume, None);
        assert!(calls[2].prompt.starts_with("TAREFA"));
        assert!(calls[2].prompt.contains("Falha 1 de 1"));

        let store = temp.open();
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.cycles, Some(2));
        assert_eq!(
            resumed_flags(&db, run_id),
            vec![
                (1, false, "initial".into()),
                (2, false, "check_failed".into())
            ]
        );
    }

    #[test]
    fn runtime_without_resume_resends_the_task_with_the_failure() {
        let agent = CycleAgent::new(false, false, 1);
        let (_temp, _run_id, _repo) = scripted_run("noresume", &agent, 3);
        let calls = agent.calls.borrow();
        assert_eq!(calls[1].resume, None);
        assert!(calls[1].prompt.starts_with("TAREFA"));
        assert!(calls[1].prompt.contains("Falha 1 de 1"));
    }

    #[test]
    fn bigger_model_cycle_continues_a_failed_run_in_its_worktree() {
        let weak = CycleAgent::new(true, false, usize::MAX);
        let (temp, run_id, repo) = scripted_run("bigger", &weak, 0);
        let store = temp.open();
        let failed = run::get(&store.connection().unwrap(), run_id).unwrap();
        assert_eq!(failed.state, run::FAILED);
        assert_eq!(failed.next_model.as_deref(), Some("sonnet"));

        let (reopened, offset) = {
            let db = store.connection().unwrap();
            (
                run::reopen(&db, run_id).unwrap(),
                run::last_cycle(&db, run_id).unwrap(),
            )
        };
        assert_eq!(offset, 1);
        let strong = CycleAgent::new(true, false, 0);
        let registry = Registry::default();
        registry.register(run_id);
        let summary = continue_with(
            scripted_job(run_id, &repo, "sonnet", 0),
            PathBuf::from(reopened.worktree_path.unwrap()),
            reopened.base_sha.unwrap(),
            offset,
            &registry,
            &DbSink { store: &store },
            &strong,
        );
        assert!(summary.passed);
        let calls = strong.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model, "sonnet");
        assert_eq!(calls[0].resume, None);
        assert!(calls[0].prompt.contains("Estado atual"));
        assert!(calls[0].prompt.contains("Falha 1 de 1"));

        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.last_model.as_deref(), Some("sonnet"));
        assert_eq!(run.next_model.as_deref(), Some("opus"));
        assert_eq!(
            resumed_flags(&db, run_id),
            vec![
                (1, false, "initial".into()),
                (2, false, "bigger_model".into())
            ]
        );
        assert!(run::reopen(&db, run_id).is_err());
    }

    /// Pasta descartável com `repo/` (git, um commit base) e `data/` (worktrees e
    /// settings da run). Apagada ao sair de escopo.
    struct TempRepo {
        root: PathBuf,
        repo: PathBuf,
        data: PathBuf,
    }

    impl TempRepo {
        fn new(label: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("harnessy-flow-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let repo = root.join("repo");
            std::fs::create_dir_all(&repo).unwrap();
            for args in [
                vec!["init", "-q"],
                vec!["config", "user.name", "teste"],
                vec!["config", "user.email", "teste@harnessy.local"],
                vec!["commit", "-q", "--allow-empty", "-m", "base"],
            ] {
                let ok = Command::new("git")
                    .args(&args)
                    .current_dir(&repo)
                    .status()
                    .unwrap()
                    .success();
                assert!(ok, "git {args:?}");
            }
            Self {
                data: root.join("data"),
                repo,
                root,
            }
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn known(input: u64, cost: f64) -> CycleUsage {
        CycleUsage {
            input: Some(input),
            output: Some(10),
            cache_read: Some(0),
            cache_write: Some(0),
            cost_usd: Some(cost),
            turns: Some(2),
        }
    }

    /// Roda uma run com dois ciclos (o check falha no primeiro) e devolve o banco.
    fn two_cycle_run(label: &str, usage: [CycleUsage; 2]) -> (TempDatabase, i64) {
        let repo = TempRepo::new(label);
        let temp = TempDatabase::new(&format!("flow-{label}"));
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Criar arquivo', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Criar arquivo",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "sonnet",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let job = Job {
            run_id,
            title: "Criar arquivo".into(),
            prompt: "Crie ok.txt.".into(),
            repo: repo.repo.clone(),
            data_dir: repo.data.clone(),
            model: "sonnet".into(),
            checks: vec![verify::Check {
                name: "Arquivo".into(),
                program: "sh".into(),
                args: vec!["-c".into(), "test -f ok.txt".into()],
                kind: verify::CheckKind::Tests,
            }],
            max_corrections: 3,
            contraprova: true,
            require_proof: false,
            effort: None,
            max_turns: crate::run::guard::MAX_TURNS,
            review: None,
            base_ref: None,
            output_schema: None,
        };
        let agent = FakeAgent {
            calls: Cell::new(0),
            usage,
        };
        let registry = Registry::default();
        registry.register(run_id);
        let summary = execute(job, &registry, &DbSink { store: &store }, &agent);
        assert!(summary.passed);
        assert_eq!(summary.agent_calls, 2);
        (temp, run_id)
    }

    #[test]
    fn two_cycles_are_recorded_and_summed() {
        let (temp, run_id) = two_cycle_run("sum", [known(100, 0.5), known(40, 0.25)]);
        let store = temp.open();
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.model.as_deref(), Some("sonnet"));
        assert_eq!(run.cycles, Some(2));
        assert_eq!(run.input_tokens, Some(140));
        assert_eq!(run.turns, Some(4));
        assert_eq!(run.cost_usd, Some(0.75));
        assert!(run.checks_ms.is_some());

        let mut query = db
            .prepare("SELECT cycle, \"trigger\", session_id FROM run_cycles WHERE run_id=?1 ORDER BY cycle")
            .unwrap();
        let rows: Vec<(i64, String, Option<String>)> = query
            .query_map([run_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (1, "initial".into(), Some(format!("sessao-{run_id}"))),
                (2, "check_failed".into(), Some(format!("sessao-{run_id}"))),
            ]
        );
    }

    #[test]
    fn a_cycle_without_usage_nulls_the_total() {
        let (temp, run_id) = two_cycle_run("null", [known(100, 0.5), CycleUsage::default()]);
        let store = temp.open();
        let db = store.connection().unwrap();
        let run = run::get(&db, run_id).unwrap();
        assert_eq!(run.cycles, Some(2));
        assert_eq!(run.input_tokens, None);
        assert_eq!(run.cost_usd, None);
        assert_eq!(run.turns, None);
    }

    /// Agente falso que só escreve os arquivos dados, em uma única chamada.
    struct ScriptedAgent {
        files: Vec<(&'static str, &'static str)>,
    }

    impl Runtime for ScriptedAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            session: &Session<'_>,
            on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            for (path, text) in &self.files {
                let target = spec.dir.join(path);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::write(target, text).unwrap();
            }
            on_event(AgentEvent::Started {
                session_id: format!("sessao-{}", session.run_id),
            });
            on_event(AgentEvent::Finished(Finish {
                ok: true,
                subtype: "success".into(),
                summary: "Pronto.".into(),
                session_id: None,
                usage: CycleUsage::default(),
                denials: 0,
                structured: None,
            }));
            Exit::Success("Pronto.".into())
        }
    }

    /// Teste que só passa com `feature.txt`; a saída segue o formato do `tsc`,
    /// para o parser extrair arquivo e linha.
    const NEEDS_CODE: &str = "test -f feature.txt || { echo \"tests/feature.test.ts(1,1): error TS2322: feature.txt ausente\"; exit 1; }\n";

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Roda uma run com um repositório cuja base tem `tests/feature.test.ts` (passa)
    /// e um agente que escreve `agent`. O check de testes é esse arquivo, rodado com `sh`.
    fn proof_run(
        label: &str,
        agent: Vec<(&'static str, &'static str)>,
    ) -> (TempRepo, TempDatabase, i64) {
        let repo = TempRepo::new(label);
        std::fs::create_dir_all(repo.repo.join("tests")).unwrap();
        std::fs::write(repo.repo.join("tests/feature.test.ts"), "exit 0\n").unwrap();
        git_in(&repo.repo, &["add", "-A"]);
        git_in(&repo.repo, &["commit", "-q", "-m", "base com teste"]);

        let temp = TempDatabase::new(&format!("flow-{label}"));
        let store = temp.open();
        let run_id = {
            let db = store.connection().unwrap();
            db.execute(
                "INSERT INTO workspaces(name, folder) VALUES ('w', ?1)",
                [repo.repo.to_string_lossy().as_ref()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO tasks(workspace_id, title, description, criteria) VALUES (1, 'Criar arquivo', '', '')",
                [],
            )
            .unwrap();
            run::create(
                &db,
                &NewRun {
                    workspace_id: 1,
                    task_id: 1,
                    title: "Criar arquivo",
                    action: "implementar",
                    role: ROLE,
                    runtime: RUNTIME,
                    model: "sonnet",
                    effort: None,
                    base_run_id: None,
                },
            )
            .unwrap()
            .id
        };
        let job = Job {
            run_id,
            title: "Criar arquivo".into(),
            prompt: "Crie feature.txt.".into(),
            repo: repo.repo.clone(),
            data_dir: repo.data.clone(),
            model: "sonnet".into(),
            checks: vec![verify::Check {
                name: "Testes".into(),
                program: "sh".into(),
                args: vec!["tests/feature.test.ts".into()],
                kind: verify::CheckKind::Tests,
            }],
            max_corrections: 3,
            contraprova: true,
            require_proof: false,
            effort: None,
            max_turns: crate::run::guard::MAX_TURNS,
            review: None,
            base_ref: None,
            output_schema: None,
        };
        let agent = ScriptedAgent { files: agent };
        let registry = Registry::default();
        registry.register(run_id);
        let summary = execute(job, &registry, &DbSink { store: &store }, &agent);
        assert!(summary.passed);
        assert_eq!(summary.agent_calls, 1);
        drop(store);
        (repo, temp, run_id)
    }

    /// Estado da run e conteúdo do commit dela, lidos da branch da run.
    fn committed(
        repo: &TempRepo,
        temp: &TempDatabase,
        run_id: i64,
        path: &str,
    ) -> (run::Run, String) {
        let store = temp.open();
        let run = {
            let db = store.connection().unwrap();
            run::get(&db, run_id).unwrap()
        };
        let branch = run.branch.clone().expect("branch da run");
        let file = git_in(&repo.repo, &["show", &format!("{branch}:{path}")]);
        (run, file)
    }

    #[test]
    fn a_test_that_fails_without_the_change_proves_it() {
        let (repo, temp, run_id) = proof_run(
            "prova",
            vec![
                ("feature.txt", "feito"),
                ("tests/feature.test.ts", NEEDS_CODE),
            ],
        );
        let (run, feature) = committed(&repo, &temp, run_id, "feature.txt");
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.proof.as_deref(), Some("proved"));
        assert_eq!(feature, "feito");
        let (_, test) = committed(&repo, &temp, run_id, "tests/feature.test.ts");
        assert_eq!(test, NEEDS_CODE);
    }

    #[test]
    fn a_test_that_already_passes_proves_nothing() {
        let (repo, temp, run_id) = proof_run(
            "nao-prova",
            vec![
                ("feature.txt", "feito"),
                ("tests/feature.test.ts", "true\n"),
            ],
        );
        let (run, feature) = committed(&repo, &temp, run_id, "feature.txt");
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.proof.as_deref(), Some("not_proved"));
        assert_eq!(feature, "feito");
    }

    #[test]
    fn a_change_without_test_files_has_no_tests() {
        let (repo, temp, run_id) = proof_run("sem-teste", vec![("feature.txt", "feito")]);
        let (run, feature) = committed(&repo, &temp, run_id, "feature.txt");
        assert_eq!(run.state, run::COMPLETED);
        assert_eq!(run.proof.as_deref(), Some("no_tests"));
        assert_eq!(feature, "feito");
    }
}
