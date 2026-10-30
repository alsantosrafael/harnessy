//! Experimento do loop de verificação: o loop aumenta a taxa de tarefas validadas
//! com o mesmo modelo, e a que custo em chamadas e prompt?
//!
//! Chama o `claude` real e gasta tokens, por isso o teste é `#[ignore]`:
//! `cargo test experiment -- --ignored --nocapture`. Variáveis opcionais:
//! `HARNESSY_EXPERIMENT_MODEL` (padrão `haiku`) e `HARNESSY_EXPERIMENT_TRIALS`
//! (padrão 1). Cada tarefa é um repositório em `experiments/loop/tasks`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicI64, Ordering},
    time::Instant,
};

use super::{actions, execute, verify, Job, Sink, Summary};
use crate::{
    action::{self, Contexto},
    process,
    run::{claude::ClaudeCode, guard, supervisor::Registry, CycleRecord},
};

const TASKS_DIR: &str = "../experiments/loop/tasks";
/// Sem loop: uma única chamada ao agente. Com loop: a correção completa.
const CONDITIONS: [(&str, u32); 2] = [("sem loop", 0), ("com loop", verify::MAX_CORRECTIONS)];
static NEXT_ID: AtomicI64 = AtomicI64::new(1_000_000);

struct Task {
    name: String,
    title: String,
    body: String,
    repo: PathBuf,
}

struct Row {
    trial: u32,
    task: String,
    condition: &'static str,
    summary: Summary,
    seconds: u64,
}

/// Mostra eventos e estados no terminal, truncados, para diagnosticar as runs.
struct Log;

impl Sink for Log {
    fn event(&self, run_id: i64, kind: &str, text: &str) {
        let line: String = text.chars().take(200).collect();
        eprintln!("    [{run_id} {kind}] {}", line.replace('\n', " "));
    }

    fn state(&self, run_id: i64, state: &str, summary: Option<&str>) {
        eprintln!("  run {run_id}: {state} {}", summary.unwrap_or(""));
    }

    fn blocked(&self, run_id: i64, reason: &str) {
        eprintln!("  run {run_id}: bloqueada: {reason}");
    }

    fn worktree(&self, _run_id: i64, _base_sha: &str, _branch: &str, _path: &str) {}

    fn cycle(&self, run_id: i64, cycle: &CycleRecord<'_>) {
        eprintln!(
            "  run {run_id}: ciclo {} · {} tokens de entrada",
            cycle.cycle,
            cycle.usage.input.map_or("?".into(), |v| v.to_string())
        );
    }

    fn checks_ms(&self, _run_id: i64, _ms: i64) {}

    fn proof(&self, run_id: i64, verdict: &str) {
        eprintln!("  run {run_id}: contraprova {verdict}");
    }
}

fn load_tasks() -> Vec<Task> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(TASKS_DIR)
        .expect("pasta de tarefas do experimento")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| {
            let text = fs::read_to_string(dir.join("task.md")).expect("task.md");
            let (heading, body) = text.split_once('\n').unwrap_or((&text, ""));
            Task {
                name: dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                title: heading.trim_start_matches('#').trim().to_owned(),
                body: body.trim().to_owned(),
                repo: dir.join("repo"),
            }
        })
        .collect()
}

/// Copia o fixture para um repositório git novo, com um commit base.
pub(super) fn fresh_repo(source: &Path, root: &Path) -> PathBuf {
    let repo = root.join("repo");
    fs::create_dir_all(&repo).expect("pasta do repositório");
    let copied = Command::new("cp")
        .arg("-R")
        .arg(format!("{}/.", source.display()))
        .arg(&repo)
        .status()
        .expect("cp");
    assert!(copied.success(), "cópia do fixture");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.name", "harnessy-experimento"]);
    git(
        &repo,
        &["config", "user.email", "experimento@harnessy.local"],
    );
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "base"]);
    repo
}

pub(super) fn git(repo: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .expect("git")
        .success();
    assert!(ok, "git {args:?}");
}

#[test]
#[ignore = "chama o claude real e gasta tokens"]
fn experiment() {
    let model = std::env::var("HARNESSY_EXPERIMENT_MODEL").unwrap_or_else(|_| "haiku".into());
    let trials: u32 = std::env::var("HARNESSY_EXPERIMENT_TRIALS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let only = std::env::var("HARNESSY_EXPERIMENT_ONLY").ok();
    let tasks: Vec<Task> = load_tasks()
        .into_iter()
        .filter(|task| only.as_deref().is_none_or(|name| task.name.contains(name)))
        .collect();
    let registry = Registry::default();
    let mut rows = Vec::new();
    // Sem Ações do usuário: o experimento mede a Ação embutida `implementar`.
    let implement = actions::find(
        &std::env::temp_dir().join("harnessy-experimento-sem-acoes"),
        None,
        action::IMPLEMENT,
    )
    .expect("ação implementar");

    for trial in 1..=trials {
        for task in &tasks {
            for (condition, max_corrections) in CONDITIONS {
                let root = std::env::temp_dir().join(format!(
                    "harnessy-exp-{}-{}",
                    std::process::id(),
                    NEXT_ID.fetch_add(1, Ordering::SeqCst)
                ));
                let repo = fresh_repo(&task.repo, &root);
                let run_id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                registry.register(run_id);
                let job = Job {
                    run_id,
                    title: task.title.clone(),
                    prompt: actions::build_prompt(
                        &implement,
                        &Contexto {
                            tarefa: &task.title,
                            descricao: &task.body,
                            criterios: "",
                            ..Default::default()
                        },
                        &[],
                    ),
                    repo: repo.clone(),
                    data_dir: root.join("data"),
                    model: model.clone(),
                    checks: verify::checks_for(&repo),
                    max_corrections,
                    contraprova: true,
                    require_proof: false,
                    effort: None,
                    max_turns: guard::MAX_TURNS,
                    review: None,
                    base_ref: None,
                    output_schema: None,
                };
                eprintln!("[{trial}] {} · {condition}", task.name);
                let started = Instant::now();
                let dirs = process::search_dirs();
                let claude = process::find_binary("claude", &dirs).expect("claude no PATH");
                let cargo_target = root.join("data").join("cargo-target");
                let runtime = ClaudeCode::new(
                    claude,
                    root.join("data"),
                    guard::env(&dirs, &cargo_target, |name| std::env::var(name).ok()),
                );
                let summary = execute(job, &registry, &Log, &runtime);
                rows.push(Row {
                    trial,
                    task: task.name.clone(),
                    condition,
                    summary,
                    seconds: started.elapsed().as_secs(),
                });
                let _ = fs::remove_dir_all(&root);
            }
        }
    }

    println!("\nModelo: {model} · tentativas: {trials}\n");
    println!("| tentativa | tarefa | condição | passou | chamadas | bytes de prompt | segundos |");
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for row in &rows {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            row.trial,
            row.task,
            row.condition,
            if row.summary.passed { "sim" } else { "não" },
            row.summary.agent_calls,
            row.summary.prompt_bytes,
            row.seconds
        );
    }
    println!();
    for (condition, _) in CONDITIONS {
        let runs: Vec<&Row> = rows
            .iter()
            .filter(|row| row.condition == condition)
            .collect();
        let passed = runs.iter().filter(|row| row.summary.passed).count();
        let calls: u32 = runs.iter().map(|row| row.summary.agent_calls).sum();
        let bytes: usize = runs.iter().map(|row| row.summary.prompt_bytes).sum();
        println!(
            "{condition}: {passed}/{} validadas ({:.0}%), {calls} chamadas, {bytes} bytes de prompt",
            runs.len(),
            100.0 * passed as f64 / runs.len().max(1) as f64
        );
    }
}
