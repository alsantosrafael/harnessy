//! Medição da spec econômica (§8): o ganho do harness para cada modelo.
//!
//! Para cada modelo, roda as mesmas tarefas em três condições — **cru** (uma
//! chamada, sem loop, sem contraprova, sem arquivos prováveis), **harness**
//! (loop, falha estruturada, contraprova, arquivos prováveis) e **harness +
//! revisor** (revisor um degrau acima, só com sinal de risco) — e aprova cada
//! resultado com **testes ocultos** que o agente nunca vê. A distância entre
//! "Pronto" (checks visíveis verdes) e "aprovada" (ocultos verdes) mede a
//! alucinação verde.
//!
//! Chama o `claude` real e gasta tokens, por isso é `#[ignore]`:
//! `cargo test economico -- --ignored --nocapture`. Variáveis opcionais:
//! `HARNESSY_ECON_MODELS` (padrão `haiku,sonnet,opus`), `HARNESSY_ECON_TRIALS`
//! (padrão 3), `HARNESSY_ECON_CONDITIONS` (padrão `cru,harness,revisor`),
//! `HARNESSY_ECON_ONLY` (filtro por nome de tarefa) e `HARNESSY_ECON_SANITY=1`
//! (inclui as 8 tarefas de `experiments/loop/tasks`, sem testes ocultos).
//! O resultado é acrescentado a `experiments/economico/RESULTADOS.md`.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicI64, Ordering},
    time::Instant,
};

use super::{
    actions, execute,
    experiment::{fresh_repo, git},
    likely, review, verify, Job, Sink,
};
use crate::{
    action::{self, Contexto},
    process,
    run::{claude::ClaudeCode, guard, supervisor::Registry, CycleRecord},
};

const TASKS_DIR: &str = "../experiments/economico/tasks";
const SANITY_DIR: &str = "../experiments/loop/tasks";
const RESULTS: &str = "../experiments/economico/RESULTADOS.md";
static NEXT_ID: AtomicI64 = AtomicI64::new(2_000_000);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Condition {
    Raw,
    Harness,
    Reviewed,
}

impl Condition {
    fn name(self) -> &'static str {
        match self {
            Self::Raw => "cru",
            Self::Harness => "harness",
            Self::Reviewed => "revisor",
        }
    }
}

struct Task {
    name: String,
    title: String,
    body: String,
    criteria: String,
    repo: PathBuf,
    /// Arquivos sobrepostos ao resultado antes da aprovação; `None` = sem ocultos.
    hidden: Option<PathBuf>,
}

/// Registra o que a medição precisa: branch, ledger de ciclos e vereditos.
#[derive(Default)]
struct Collect {
    branch: RefCell<Option<String>>,
    state: RefCell<String>,
    input: RefCell<u64>,
    output: RefCell<u64>,
    cache_read: RefCell<u64>,
    cost: RefCell<f64>,
    missing_usage: RefCell<bool>,
    cycles: RefCell<u32>,
    proof: RefCell<String>,
    review: RefCell<String>,
}

impl Sink for Collect {
    fn event(&self, run_id: i64, kind: &str, text: &str) {
        if kind == "status" {
            let line: String = text.chars().take(160).collect();
            eprintln!("    [{run_id}] {line}");
        }
    }
    fn state(&self, _: i64, state: &str, _: Option<&str>) {
        *self.state.borrow_mut() = state.to_owned();
    }
    fn blocked(&self, _: i64, _: &str) {}
    fn worktree(&self, _: i64, _: &str, branch: &str, _: &str) {
        *self.branch.borrow_mut() = Some(branch.to_owned());
    }
    fn cycle(&self, _: i64, cycle: &CycleRecord<'_>) {
        *self.cycles.borrow_mut() += 1;
        let usage = cycle.usage;
        match (usage.input, usage.output, usage.cost_usd) {
            (Some(input), Some(output), Some(cost)) => {
                *self.input.borrow_mut() += input;
                *self.output.borrow_mut() += output;
                *self.cache_read.borrow_mut() += usage.cache_read.unwrap_or(0);
                *self.cost.borrow_mut() += cost;
            }
            _ => *self.missing_usage.borrow_mut() = true,
        }
    }
    fn checks_ms(&self, _: i64, _: i64) {}
    fn proof(&self, _: i64, verdict: &str) {
        *self.proof.borrow_mut() = verdict.to_owned();
    }
    fn review(&self, _: i64, verdict: &str, _: usize) {
        *self.review.borrow_mut() = verdict.to_owned();
    }
}

struct Row {
    model: String,
    condition: Condition,
    task: String,
    ready: bool,
    approved: bool,
    tokens: u64,
    cache_read: u64,
    cost: f64,
    missing_usage: bool,
    cycles: u32,
    seconds: u64,
    proof: String,
    review: String,
}

fn load(dir: &str, with_hidden: bool) -> Vec<Task> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("task.md").is_file())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| {
            let text = fs::read_to_string(dir.join("task.md")).expect("task.md");
            let (heading, rest) = text.split_once('\n').unwrap_or((&text, ""));
            let (body, criteria) = rest
                .split_once("## Critérios de aceitação")
                .unwrap_or((rest, ""));
            Task {
                name: dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                title: heading.trim_start_matches('#').trim().to_owned(),
                body: body.trim().to_owned(),
                criteria: criteria.trim().to_owned(),
                repo: dir.join("repo"),
                hidden: Some(dir.join("hidden")).filter(|path| with_hidden && path.is_dir()),
            }
        })
        .collect()
}

/// Testes ocultos: o resultado da run (branch) recebe os arquivos de `hidden/`
/// e precisa passar em todas as verificações do projeto. Sem branch, reprovado.
fn approve(repo: &Path, branch: Option<&str>, hidden: Option<&Path>, ready: bool) -> bool {
    let Some(hidden) = hidden else {
        return ready;
    };
    let Some(branch) = branch else {
        return false;
    };
    let exists = Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .current_dir(repo)
        .output()
        .is_ok_and(|out| out.status.success());
    if !exists {
        return false;
    }
    let check = repo.with_file_name("grader");
    let _ = fs::remove_dir_all(&check);
    git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &check.to_string_lossy(),
            branch,
        ],
    );
    let copied = Command::new("cp")
        .arg("-R")
        .arg(format!("{}/.", hidden.display()))
        .arg(&check)
        .status()
        .is_ok_and(|status| status.success());
    let dirs = process::search_dirs();
    let passed = copied
        && verify::run_all(&verify::checks_for(&check), &dirs, &check)
            .is_ok_and(|reports| !reports.is_empty() && reports.iter().all(|r| r.passed));
    git(
        repo,
        &["worktree", "remove", "--force", &check.to_string_lossy()],
    );
    passed
}

fn env_list(name: &str, default: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_else(|_| default.to_owned())
        .split(',')
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

fn command_line(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default()
}

#[test]
#[ignore = "chama o claude real e gasta tokens"]
fn economico() {
    let models = env_list("HARNESSY_ECON_MODELS", "haiku,sonnet,opus");
    for model in &models {
        guard::model(model).expect("modelo da lista do guard");
    }
    let conditions: Vec<Condition> = env_list("HARNESSY_ECON_CONDITIONS", "cru,harness,revisor")
        .iter()
        .map(|name| match name.as_str() {
            "cru" => Condition::Raw,
            "harness" => Condition::Harness,
            "revisor" => Condition::Reviewed,
            other => panic!("condição desconhecida: {other}"),
        })
        .collect();
    let trials: u32 = std::env::var("HARNESSY_ECON_TRIALS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3);
    let only = std::env::var("HARNESSY_ECON_ONLY").ok();
    let mut tasks = load(TASKS_DIR, true);
    if std::env::var("HARNESSY_ECON_SANITY").is_ok() {
        tasks.extend(load(SANITY_DIR, false));
    }
    tasks.retain(|task| only.as_deref().is_none_or(|name| task.name.contains(name)));
    assert!(!tasks.is_empty(), "nenhuma tarefa em {TASKS_DIR}");

    let no_actions = std::env::temp_dir().join("harnessy-econ-sem-acoes");
    let implement = actions::find(&no_actions, None, action::IMPLEMENT).expect("implementar");
    let reviewer_action = actions::find(&no_actions, None, action::REVIEW).expect("revisar");
    let dirs = process::search_dirs();
    let claude = process::find_binary("claude", &dirs).expect("claude no PATH");
    let registry = Registry::default();
    let mut rows = Vec::new();

    for trial in 1..=trials {
        for task in &tasks {
            for model in &models {
                for &condition in &conditions {
                    let root = std::env::temp_dir().join(format!(
                        "harnessy-econ-{}-{}",
                        std::process::id(),
                        NEXT_ID.fetch_add(1, Ordering::SeqCst)
                    ));
                    let repo = fresh_repo(&task.repo, &root);
                    let likely_files = if condition == Condition::Raw {
                        String::new()
                    } else {
                        likely::find(&repo, &task.title, &task.criteria)
                    };
                    let prompt = actions::build_prompt(
                        &implement,
                        &Contexto {
                            tarefa: &task.title,
                            descricao: &task.body,
                            criterios: &task.criteria,
                            arquivos_provaveis: &likely_files,
                            ..Default::default()
                        },
                        &[],
                    );
                    let run_id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                    registry.register(run_id);
                    let harness = condition != Condition::Raw;
                    let job = Job {
                        run_id,
                        title: task.title.clone(),
                        prompt,
                        repo: repo.clone(),
                        data_dir: root.join("data"),
                        model: model.clone(),
                        checks: verify::checks_for(&repo),
                        max_corrections: if harness { verify::MAX_CORRECTIONS } else { 0 },
                        contraprova: harness,
                        require_proof: false,
                        effort: None,
                        max_turns: guard::MAX_TURNS,
                        review: (condition == Condition::Reviewed).then(|| review::Reviewer {
                            model: review::Reviewer::model_for(&reviewer_action, model),
                            action: reviewer_action.clone(),
                            tarefa: task.title.clone(),
                            descricao: task.body.clone(),
                            criterios: task.criteria.clone(),
                            runtime: None,
                        }),
                        base_ref: None,
                        output_schema: None,
                    };
                    eprintln!("[{trial}] {} · {model} · {}", task.name, condition.name());
                    let runtime = ClaudeCode::new(
                        claude.clone(),
                        root.join("data"),
                        guard::env(&dirs, &root.join("data/cargo-target"), |name| {
                            std::env::var(name).ok()
                        }),
                    );
                    let sink = Collect::default();
                    let started = Instant::now();
                    let summary = execute(job, &registry, &sink, &runtime);
                    let ready = summary.passed && *sink.state.borrow() == "completed";
                    let approved = approve(
                        &repo,
                        sink.branch.borrow().as_deref(),
                        task.hidden.as_deref(),
                        ready,
                    );
                    rows.push(Row {
                        model: model.clone(),
                        condition,
                        task: task.name.clone(),
                        ready,
                        approved,
                        tokens: *sink.input.borrow() + *sink.output.borrow(),
                        cache_read: *sink.cache_read.borrow(),
                        cost: *sink.cost.borrow(),
                        missing_usage: *sink.missing_usage.borrow(),
                        cycles: *sink.cycles.borrow(),
                        seconds: started.elapsed().as_secs(),
                        proof: sink.proof.borrow().clone(),
                        review: sink.review.borrow().clone(),
                    });
                    let _ = fs::remove_dir_all(&root);
                }
            }
        }
    }

    let report = report(&rows, &tasks, trials);
    println!("\n{report}");
    let previous = fs::read_to_string(RESULTS).unwrap_or_default();
    fs::write(RESULTS, format!("{previous}\n{report}")).expect("RESULTADOS.md");
}

fn report(rows: &[Row], tasks: &[Task], trials: u32) -> String {
    let date = command_line("date", &["+%Y-%m-%d %H:%M"]);
    let cli = command_line("claude", &["--version"]);
    let mut out = format!(
        "## Medição {date}\n\nBack {} · CLI {cli} · {} tarefa(s) · {trials} tentativa(s) por célula.\n\n",
        env!("CARGO_PKG_VERSION"),
        tasks.len()
    );
    out.push_str("| modelo | condição | aprovadas | Pronto | tokens/aprovada | US$/aprovada | ciclos médios | leitura de cache | p50 s | sem uso informado |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    let mut cells: BTreeMap<(String, Condition), Vec<&Row>> = BTreeMap::new();
    for row in rows {
        cells
            .entry((row.model.clone(), row.condition))
            .or_default()
            .push(row);
    }
    let mut per_model: BTreeMap<String, BTreeMap<Condition, (f64, Option<f64>)>> = BTreeMap::new();
    for ((model, condition), cell) in &cells {
        let n = cell.len() as f64;
        let approved = cell.iter().filter(|row| row.approved).count();
        let ready = cell.iter().filter(|row| row.ready).count();
        let tokens: u64 = cell.iter().map(|row| row.tokens).sum();
        let cache: u64 = cell.iter().map(|row| row.cache_read).sum();
        let cost: f64 = cell.iter().map(|row| row.cost).sum();
        let missing = cell.iter().filter(|row| row.missing_usage).count();
        let cycles: u32 = cell.iter().map(|row| row.cycles).sum();
        let mut seconds: Vec<u64> = cell.iter().map(|row| row.seconds).collect();
        seconds.sort_unstable();
        let per_approved = |total: f64| (approved > 0).then(|| total / approved as f64);
        let tokens_per = per_approved(tokens as f64);
        per_model
            .entry(model.clone())
            .or_default()
            .insert(*condition, (approved as f64 / n, tokens_per));
        out.push_str(&format!(
            "| {model} | {} | {approved}/{} ({:.0}%) | {ready}/{} | {} | {} | {:.1} | {:.0}% | {} | {missing} |\n",
            condition.name(),
            cell.len(),
            100.0 * approved as f64 / n,
            cell.len(),
            tokens_per.map_or("—".into(), |v| format!("{v:.0}")),
            per_approved(cost).map_or("—".into(), |v| format!("{v:.3}")),
            cycles as f64 / n,
            100.0 * cache as f64 / (tokens + cache).max(1) as f64,
            seconds[seconds.len() / 2],
        ));
    }
    out.push_str("\n### Ganho do harness por modelo\n\n| modelo | aprovação (harness − cru) | tokens por aprovada (harness ÷ cru) |\n| --- | --- | --- |\n");
    for (model, by) in &per_model {
        if let (Some(raw), Some(harness)) = (by.get(&Condition::Raw), by.get(&Condition::Harness)) {
            let ratio = match (raw.1, harness.1) {
                (Some(r), Some(h)) if r > 0.0 => format!("{:.2}", h / r),
                _ => "—".into(),
            };
            out.push_str(&format!(
                "| {model} | {:+.0} p.p. | {ratio} |\n",
                100.0 * (harness.0 - raw.0)
            ));
        }
    }
    out.push_str("\n### Por tarefa\n\n| tarefa | modelo | condição | Pronto | aprovada | ciclos | contraprova | revisão |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            row.task,
            row.model,
            row.condition.name(),
            if row.ready { "sim" } else { "não" },
            if row.approved { "sim" } else { "não" },
            row.cycles,
            if row.proof.is_empty() {
                "—"
            } else {
                &row.proof
            },
            if row.review.is_empty() {
                "—"
            } else {
                &row.review
            },
        ));
    }
    out
}

#[test]
fn tasks_with_hidden_tests_load_and_their_reference_solutions_pass() {
    // Cada tarefa traz `solution/`: aplicada sobre o repositório, os testes
    // visíveis e os ocultos passam; sem ela, os ocultos falham. Garante que a
    // suíte mede o que diz medir, sem chamar modelo.
    let dirs = process::search_dirs();
    if process::find_binary("bun", &dirs).is_none() {
        return;
    }
    let tasks = load(TASKS_DIR, true);
    assert!(tasks.len() >= 3, "tarefas reais em {TASKS_DIR}");
    for task in &tasks {
        let hidden = task.hidden.as_deref().expect("hidden/");
        let solution = task.repo.with_file_name("solution");
        assert!(solution.is_dir(), "{}: falta solution/", task.name);
        let root = std::env::temp_dir().join(format!(
            "harnessy-econ-ref-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::SeqCst)
        ));
        let repo = fresh_repo(&task.repo, &root);
        git(&repo, &["checkout", "-q", "-b", "referencia"]);
        let ok = Command::new("cp")
            .arg("-R")
            .arg(format!("{}/.", solution.display()))
            .arg(&repo)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "solução de referência"]);
        assert!(
            approve(&repo, Some("referencia"), Some(hidden), true),
            "{}: solução de referência reprovada",
            task.name
        );
        git(&repo, &["checkout", "-q", "-b", "sem-mudanca", "HEAD~1"]);
        assert!(
            !approve(&repo, Some("sem-mudanca"), Some(hidden), true),
            "{}: ocultos passam sem a mudança",
            task.name
        );
        let _ = fs::remove_dir_all(&root);
    }
}
