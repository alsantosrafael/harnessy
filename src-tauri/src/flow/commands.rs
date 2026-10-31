//! Adaptadores IPC que cruzam domínios: prévia do prompt e início de uma run.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter, Manager, State};

use super::{
    actions, continue_with, execute, likely, read, review::Reviewer, review_existing, sync, verify,
    Job, Sink, ROLE,
};
use crate::{
    action::{self, Acesso, Alvo, Contexto, Quando, Repositorio, Saida},
    db::Database,
    error::{AppError, Result},
    process,
    run::{
        self, claude::ClaudeCode, codex::Codex, guard, prompt, redact, runtime::Runtime,
        supervisor::Registry, worktree, CycleRecord, NewRun, Run,
    },
    task::{self, commands::emit_change, Task},
    workspace,
};

const MAX_TITLE_BYTES: usize = 300;
/// Quanto do pedido do usuário aparece na conversa da run.
const MAX_SAID_CHARS: usize = 4_000;

/// Ações do repositório do workspace e os hashes que o usuário já confirmou.
fn repo_catalog(
    store: &Database,
    ws: &workspace::Workspace,
) -> Result<(PathBuf, HashMap<String, String>)> {
    let aprovados = action::aprovacoes(&*store.connection()?, ws.id)?;
    Ok((action::repo_dir(Path::new(&ws.folder)), aprovados))
}

/// Pasta de dados do app; dentro dela ficam as Ações do usuário.
fn app_data(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|_| AppError::Validation("Pasta de dados indisponível."))
}

fn contexto<'a>(task: &'a Task, arquivos: &'a str) -> Contexto<'a> {
    Contexto {
        tarefa: &task.title,
        descricao: &task.description,
        criterios: &task.criteria,
        arquivos_provaveis: arquivos,
        ..Default::default()
    }
}

/// Escolha de revisão vinda da interface, já validada: agente, modelo
/// (vazio = automático, só no Claude Code) e quando revisar.
struct ReviewPick {
    runtime: &'static str,
    model: String,
    when: &'static str,
}

impl ReviewPick {
    fn parse(runtime: &str, model: &str, when: &str) -> Result<Self> {
        let runtime = guard::runtime(runtime)?;
        let model = if runtime == "claude" && model.trim().is_empty() {
            String::new()
        } else {
            guard::agent_model(runtime, model)?
        };
        Ok(Self {
            runtime,
            model,
            when: guard::review_when(when)?,
        })
    }
}

/// Runtime de um agente. O ambiente é a lista fixa de [`guard::env`] (regra 26);
/// binário ausente recusa antes de criar a run.
fn agent_runtime(kind: &str, dirs: &[PathBuf], data_dir: &Path) -> Result<Box<dyn Runtime + Send>> {
    let env = guard::env(dirs, &data_dir.join("cargo-target"), |name| {
        std::env::var(name).ok()
    });
    match kind {
        "codex" => {
            let binary = process::find_binary("codex", dirs).ok_or(AppError::Validation(
                "Codex não encontrado neste computador.",
            ))?;
            Ok(Box::new(Codex::new(binary, data_dir.to_path_buf(), env)))
        }
        _ => {
            let binary = process::find_binary("claude", dirs).ok_or(AppError::Validation(
                "Claude Code não encontrado neste computador.",
            ))?;
            Ok(Box::new(ClaudeCode::new(
                binary,
                data_dir.to_path_buf(),
                env,
            )))
        }
    }
}

/// Revisor da run de escrita: a Ação `revisar` do catálogo (a do usuário vence),
/// salvo quando ela está inválida, não é sobre Run, está em `quando = "manual"`
/// ou a interface pediu "nunca". "Sempre" vence o `quando` da Ação.
/// Sem modelo escolhido, o Claude Code revisa um degrau acima do implementador.
fn reviewer_for(
    data_dir: &Path,
    repo: Option<&Repositorio<'_>>,
    task: &Task,
    implementer_model: &str,
    pick: &ReviewPick,
    runtime: Box<dyn Runtime + Send>,
    harness: String,
) -> Option<Reviewer> {
    if pick.when == "never" {
        return None;
    }
    let mut acao = actions::find(data_dir, repo, action::REVIEW).ok()?;
    if acao.alvo != Alvo::Run || acao.saida != Saida::Revisao {
        return None;
    }
    match pick.when {
        "always" => acao.quando = Quando::Sempre,
        _ if acao.quando == Quando::Manual => return None,
        _ => {}
    }
    let model = if pick.model.is_empty() {
        Reviewer::model_for(&acao, implementer_model)
    } else {
        pick.model.clone()
    };
    Some(Reviewer {
        model,
        runtime: Some(runtime),
        action: acao,
        tarefa: task.title.clone(),
        descricao: task.description.clone(),
        criterios: task.criteria.clone(),
        harness,
    })
}

/// Prompt de uma Ação sobre uma Tarefa: arquivos prováveis da pasta do workspace
/// e comandos que o Harnessy já negou ali.
fn prompt_for(acao: &action::Action, task: &Task, folder: &str, avoid: &[String]) -> String {
    let arquivos = likely::find(Path::new(folder), &task.title, &task.criteria);
    let harness = project_harness(folder, "HEAD");
    actions::build_prompt(acao, &contexto(task, &arquivos), avoid, &harness)
}

/// Harness do projeto no commit `rev`, em bloco de dados para o prompt. Lido pelo Rust do
/// commit base (nunca do worktree da run); sem git ou sem arquivos, vazio.
fn project_harness(folder: &str, rev: &str) -> String {
    let dirs = process::search_dirs();
    let Some(git) = process::find_binary("git", &dirs) else {
        return String::new();
    };
    let files = run::harness::load(&git, &dirs, Path::new(folder), rev);
    run::harness::frame(rev, &files)
}

/// Texto exato que o agente receberá, para o usuário conferir antes de iniciar.
#[tauri::command]
pub fn preview_prompt(
    app: AppHandle,
    store: State<'_, Database>,
    workspace_id: i64,
    task_id: i64,
    action: String,
) -> Result<String> {
    let (ws, task, avoid) = {
        let db = store.connection()?;
        (
            workspace::get(&db, workspace_id)?,
            task::get(&db, workspace_id, task_id)?,
            run::top_denials(&db, workspace_id)?,
        )
    };
    let (dir, aprovados) = repo_catalog(&store, &ws)?;
    let repo = Repositorio {
        dir,
        aprovados: &aprovados,
    };
    let acao = actions::find(&app_data(&app)?, Some(&repo), &action)?;
    Ok(prompt_for(&acao, &task, &ws.folder, &avoid))
}

/// Ações para o menu da tarefa, com erro quando alguma está inválida. Com
/// `workspace_id`, confere que o workspace existe.
#[tauri::command]
pub fn list_actions(
    app: AppHandle,
    store: State<'_, Database>,
    workspace_id: Option<i64>,
) -> Result<Vec<action::Resumo>> {
    let data_dir = app_data(&app)?;
    let entradas = match workspace_id {
        Some(id) => {
            let ws = workspace::get(&*store.connection()?, id)?;
            let (dir, aprovados) = repo_catalog(&store, &ws)?;
            let repo = Repositorio {
                dir,
                aprovados: &aprovados,
            };
            actions::load(&data_dir, Some(&repo))
        }
        None => actions::load(&data_dir, None),
    };
    Ok(entradas.iter().map(|entrada| entrada.resumo()).collect())
}

/// Confirma o conteúdo atual de uma Ação do repositório (SPEC §15): só depois
/// disso ela roda. Alterar o arquivo exige nova confirmação.
#[tauri::command]
pub fn approve_action(
    app: AppHandle,
    store: State<'_, Database>,
    workspace_id: i64,
    id: String,
) -> Result<()> {
    let ws = workspace::get(&*store.connection()?, workspace_id)?;
    let (dir, aprovados) = repo_catalog(&store, &ws)?;
    let repo = Repositorio {
        dir,
        aprovados: &aprovados,
    };
    let (arquivo, hash) = actions::load(&app_data(&app)?, Some(&repo))
        .into_iter()
        .find(|entrada| entrada.id == id)
        .and_then(|entrada| entrada.pendente)
        .ok_or(AppError::Validation(
            "Não há Ação do repositório aguardando confirmação com este id.",
        ))?;
    action::aprovar(&*store.connection()?, workspace_id, &arquivo, &hash)
}

/// Cria uma Ação do usuário a partir do formulário: valida com o mesmo parser
/// dos arquivos e grava em `<dados do app>/actions/<id>.toml`.
#[tauri::command]
pub fn create_action(app: AppHandle, input: action::Nova) -> Result<action::Resumo> {
    let (text, acao) = action::texto_da_nova(&input)?;
    if let Some(modelo) = acao.modelo.as_deref() {
        guard::model(modelo)?;
    }
    let data_dir = app_data(&app)?;
    action::gravar_nova(&action::user_dir(&data_dir), &acao.id, &text)?;
    actions::load(&data_dir, None)
        .iter()
        .find(|entrada| entrada.id == acao.id)
        .map(|entrada| entrada.resumo())
        .ok_or(AppError::Validation(
            "A Ação foi gravada, mas não apareceu no catálogo.",
        ))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn start_run(
    app: AppHandle,
    store: State<'_, Database>,
    registry: State<'_, Registry>,
    workspace_id: i64,
    task_id: i64,
    runtime: String,
    model: String,
    action: String,
    continue_from: Option<i64>,
    reviewer: String,
    reviewer_model: String,
    review_when: String,
    instruction: Option<String>,
) -> Result<Run> {
    let runtime_id = guard::runtime(&runtime)?;
    let model = guard::agent_model(runtime_id, &model)?;
    let pick = ReviewPick::parse(&reviewer, &reviewer_model, &review_when)?;
    let (ws, task, avoid) = {
        let db = store.connection()?;
        (
            workspace::get(&db, workspace_id)?,
            task::get(&db, workspace_id, task_id)?,
            run::top_denials(&db, workspace_id)?,
        )
    };
    let (repo_dir, aprovados) = repo_catalog(&store, &ws)?;
    let repo = Repositorio {
        dir: repo_dir,
        aprovados: &aprovados,
    };
    let acao = actions::find(&app_data(&app)?, Some(&repo), &action)?;
    if acao.alvo != Alvo::Tarefa || acao.saida == Saida::Revisao {
        return Err(AppError::Validation(
            "Esta Ação não roda como run de tarefa.",
        ));
    }
    let read_only = acao.acesso == Acesso::Leitura;
    // Passo encadeado: a run parte da branch de uma run concluída do mesmo workspace.
    let base = match continue_from.filter(|_| !read_only) {
        Some(previous) => {
            let previous = run::get(&*store.connection()?, previous)?;
            match (&previous.branch, previous.state.as_str()) {
                (Some(branch), run::COMPLETED) if previous.workspace_id == workspace_id => {
                    Some((previous.id, branch.clone()))
                }
                _ => {
                    return Err(AppError::Validation(
                        "Só dá para continuar de uma run concluída deste workspace.",
                    ))
                }
            }
        }
        None => None,
    };
    if ws.isolation != "none" {
        return Err(AppError::Validation(
            "Este workspace exige um modo de isolamento ainda não implementado.",
        ));
    }
    process::wait_for_login_path();
    let dirs = process::search_dirs();
    // Leitura roda na pasta do workspace, sem worktree, então não precisa de git.
    if !read_only && process::find_binary("git", &dirs).is_none() {
        return Err(AppError::Validation("git não encontrado neste computador."));
    }
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Run("pasta de dados indisponível.".into()))?;
    let agent = agent_runtime(runtime_id, &dirs, &data_dir)?;
    // Só run de escrita é revisada; agente ausente recusa antes de criar a run.
    let review_runtime = if read_only {
        None
    } else {
        Some(agent_runtime(pick.runtime, &dirs, &data_dir)?)
    };

    // O prompt (e a validação do ajuste) vem antes de criar a run: recusar depois deixaria
    // uma run `preparing` sem processo, que trava o workspace.
    let prompt = prompt::with_followup(
        prompt_for(&acao, &task, &ws.folder, &avoid),
        instruction.as_deref().unwrap_or(""),
        base.is_some(),
    )?;
    let title: String = task.title.chars().take(MAX_TITLE_BYTES / 4).collect();
    let created = run::create(
        &*store.connection()?,
        &NewRun {
            workspace_id,
            task_id: task.id,
            title: &title,
            action: &acao.id,
            role: ROLE,
            runtime: runtime_id,
            model: &model,
            effort: acao.esforco.as_deref(),
            base_run_id: base.as_ref().map(|(id, _)| *id),
        },
    )?;
    if !read_only {
        run::set_review_choice(
            &*store.connection()?,
            created.id,
            pick.runtime,
            &pick.model,
            pick.when,
        )?;
    }
    registry.register(created.id);
    // Leitura não move o card: a resposta é opinião do agente, não evidência.
    if !read_only {
        sync_task(&app, &created);
    }

    let data_dir_review = app_data(&app)?;
    let checks = if read_only {
        Vec::new()
    } else {
        verify::checks_for(Path::new(&ws.folder))
    };
    let said: String = match instruction
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        Some(text) => text.to_owned(),
        None if task.description.trim().is_empty() => task.title.clone(),
        None => format!("{}\n\n{}", task.title, task.description.trim()),
    }
    .chars()
    .take(MAX_SAID_CHARS)
    .collect();
    let job = Job {
        run_id: created.id,
        title,
        prompt,
        repo: ws.folder.clone().into(),
        data_dir,
        model: model.clone(),
        checks,
        max_corrections: if read_only {
            0
        } else {
            verify::MAX_CORRECTIONS
        },
        contraprova: acao.contraprova,
        require_proof: acao.exigir_prova,
        effort: acao.esforco.clone(),
        max_turns: actions::max_turns(&acao),
        review: review_runtime.and_then(|runtime| {
            reviewer_for(
                &data_dir_review,
                Some(&repo),
                &task,
                &model,
                &pick,
                runtime,
                project_harness(&ws.folder, "HEAD"),
            )
        }),
        base_ref: base.map(|(_, branch)| branch),
        output_schema: (acao.saida == Saida::Passos).then_some(read::STEPS_SCHEMA),
    };
    let handle = app.clone();
    let sink = AppSink {
        app: app.clone(),
        read_only,
    };
    // O pedido de quem usa abre a conversa da run: o ajuste digitado ou a própria tarefa.
    sink.event(created.id, "user", &said);
    std::thread::spawn(move || {
        let registry = handle.state::<Registry>();
        if read_only {
            read::execute(job, &registry, &sink, agent.as_ref());
        } else {
            execute(job, &registry, &sink, agent.as_ref());
        }
    });
    Ok(created)
}

/// Ciclo extra numa run que falhou, com o próximo modelo da lista acima do
/// último ciclo. A WebView só manda o id; modelo e prompt são decididos aqui.
#[tauri::command]
pub fn continue_run(
    app: AppHandle,
    store: State<'_, Database>,
    registry: State<'_, Registry>,
    run_id: i64,
) -> Result<Run> {
    let previous = run::get(&*store.connection()?, run_id)?;
    let model = previous.next_model.clone().ok_or(AppError::Validation(
        "Esta run já usou o modelo mais forte da lista.",
    ))?;
    let model = guard::model(&model)?;
    let (ws, task, avoid) = {
        let db = store.connection()?;
        (
            workspace::get(&db, previous.workspace_id)?,
            task::get(&db, previous.workspace_id, previous.task_id)?,
            run::top_denials(&db, previous.workspace_id)?,
        )
    };
    let (repo_dir, aprovados) = repo_catalog(&store, &ws)?;
    let repo = Repositorio {
        dir: repo_dir,
        aprovados: &aprovados,
    };
    let acao = actions::find(&app_data(&app)?, Some(&repo), &previous.action)?;
    if acao.acesso == Acesso::Leitura {
        return Err(AppError::Validation(
            "Ação de leitura não ganha ciclo extra.",
        ));
    }
    process::wait_for_login_path();
    let dirs = process::search_dirs();
    let data_dir = app_data(&app)?;
    let agent = agent_runtime(guard::runtime(&previous.runtime)?, &dirs, &data_dir)?;
    // O revisor escolhido ao iniciar a run vale também para o ciclo extra.
    let pick = ReviewPick::parse(
        previous.reviewer.as_deref().unwrap_or("claude"),
        previous.reviewer_model.as_deref().unwrap_or(""),
        previous.review_when.as_deref().unwrap_or("auto"),
    )?;
    let review_runtime = agent_runtime(pick.runtime, &dirs, &data_dir)?;
    let (reopened, offset) = {
        let db = store.connection()?;
        let reopened = run::reopen(&db, run_id)?;
        (reopened, run::last_cycle(&db, run_id)?)
    };
    registry.register(run_id);
    sync_task(&app, &reopened);
    let dir = PathBuf::from(reopened.worktree_path.clone().unwrap_or_default());
    let base = reopened.base_sha.clone().unwrap_or_default();
    let job = Job {
        run_id,
        title: reopened.title.clone(),
        prompt: prompt_for(&acao, &task, &ws.folder, &avoid),
        repo: ws.folder.clone().into(),
        data_dir,
        model: model.into(),
        checks: verify::checks_for(&dir),
        max_corrections: 0,
        contraprova: acao.contraprova,
        require_proof: acao.exigir_prova,
        effort: acao.esforco.clone(),
        max_turns: actions::max_turns(&acao),
        review: reviewer_for(
            &app_data(&app)?,
            Some(&repo),
            &task,
            model,
            &pick,
            review_runtime,
            project_harness(&ws.folder, "HEAD"),
        ),
        base_ref: None,
        output_schema: None,
    };
    let handle = app.clone();
    let sink = AppSink {
        app: app.clone(),
        read_only: false,
    };
    std::thread::spawn(move || {
        let registry = handle.state::<Registry>();
        continue_with(job, dir, base, offset, &registry, &sink, agent.as_ref());
    });
    Ok(reopened)
}

/// Revisão sob demanda de uma run concluída, com o revisor escolhido na interface
/// (sempre roda, qualquer que seja o `quando` da Ação). A resposta chega pelos
/// eventos `run-event` e `run-state`.
#[tauri::command]
pub fn review_run(
    app: AppHandle,
    store: State<'_, Database>,
    registry: State<'_, Registry>,
    run_id: i64,
    reviewer: String,
    reviewer_model: String,
) -> Result<Run> {
    let (previous, ws, task, number) = {
        let db = store.connection()?;
        let previous = run::get(&db, run_id)?;
        sync::ensure_reviewable(&db, &previous)?;
        let ws = workspace::get(&db, previous.workspace_id)?;
        let task = task::get(&db, previous.workspace_id, previous.task_id)?;
        let number = run::last_cycle(&db, run_id)? + 1;
        (previous, ws, task, number)
    };
    let pick = ReviewPick::parse(&reviewer, &reviewer_model, "always")?;
    process::wait_for_login_path();
    let dirs = process::search_dirs();
    let data_dir = app_data(&app)?;
    let runtime = agent_runtime(pick.runtime, &dirs, &data_dir)?;
    let (repo_dir, aprovados) = repo_catalog(&store, &ws)?;
    let repo = Repositorio {
        dir: repo_dir,
        aprovados: &aprovados,
    };
    let implementer = previous.model.clone().unwrap_or_default();
    // A revisão sob demanda lê as regras do commit base da run, não da branch revisada.
    let base_harness = project_harness(&ws.folder, previous.base_sha.as_deref().unwrap_or("HEAD"));
    let reviewer = reviewer_for(
        &data_dir,
        Some(&repo),
        &task,
        &implementer,
        &pick,
        runtime,
        base_harness,
    )
    .ok_or(AppError::Validation(
        "A Ação de revisão não está disponível.",
    ))?;
    let (base, branch) = (
        previous.base_sha.clone().unwrap_or_default(),
        previous.branch.clone().unwrap_or_default(),
    );
    let proof = previous
        .proof
        .clone()
        .unwrap_or_else(|| "não registrada".into());
    if !registry.try_register(run_id) {
        return Err(AppError::Validation("Esta run já está sendo revisada."));
    }
    let handle = app.clone();
    let folder = PathBuf::from(&ws.folder);
    let sink = AppSink {
        app: app.clone(),
        read_only: false,
    };
    std::thread::spawn(move || {
        let registry = handle.state::<Registry>();
        let Some(runtime) = reviewer.runtime.as_deref() else {
            registry.unregister(run_id);
            return;
        };
        review_existing(
            run_id, &reviewer, &folder, &base, &branch, &proof, number, &registry, &sink, runtime,
        );
        let store = handle.state::<Database>();
        if let Ok(updated) = store.connection().and_then(|db| run::get(&db, run_id)) {
            let _ = handle.emit("run-state", updated);
        }
    });
    Ok(previous)
}

/// Exclui uma tarefa (e as runs dela) a pedido do usuário.
#[tauri::command]
pub fn delete_task(
    store: State<'_, Database>,
    previews: State<'_, run::preview::Previews>,
    workspace_id: i64,
    task_id: i64,
) -> Result<()> {
    let (folder, worktrees) = {
        let db = store.connection()?;
        // Um ambiente de teste rodando num worktree que vai sumir seria um servidor órfão.
        for run in run::list(&db, workspace_id)?
            .iter()
            .filter(|run| run.task_id == task_id)
        {
            previews.stop(run.id);
        }
        (
            workspace::get(&db, workspace_id)?.folder,
            run::worktrees_of_task(&db, task_id)?,
        )
    };
    sync::remove_task(&*store.connection()?, workspace_id, task_id)?;
    remove_leftover_worktrees(Path::new(&folder), &worktrees);
    Ok(())
}

/// Sem as runs no banco, ninguém mais usaria estas pastas: remove as que sobraram.
/// Best effort; a branch continua no git.
fn remove_leftover_worktrees(repo: &Path, worktrees: &[String]) {
    let dirs = process::search_dirs();
    let Some(git) = process::find_binary("git", &dirs) else {
        return;
    };
    for dir in worktrees.iter().map(Path::new).filter(|dir| dir.is_dir()) {
        let _ = worktree::remove(&git, &dirs, repo, dir);
    }
}

/// O usuário deu o bloqueio de uma run por resolvido (regras em [`sync::resolve`]).
#[tauri::command]
pub fn resolve_block(app: AppHandle, store: State<'_, Database>, run_id: i64) -> Result<()> {
    let (run, change) = sync::resolve(&*store.connection()?, run_id)?;
    if let Some(change) = &change {
        emit_change(&app, change);
        let _ = app.emit("run-state", run);
    }
    Ok(())
}

struct AppSink {
    app: AppHandle,
    /// Ação de leitura: o estado da run não move o card.
    read_only: bool,
}

impl Sink for AppSink {
    fn event(&self, run_id: i64, kind: &str, text: &str) {
        let text = redact::clean(text);
        let store = self.app.state::<Database>();
        let saved = store
            .connection()
            .and_then(|db| run::append_event(&db, run_id, kind, &text));
        if let Ok(Some(event)) = saved {
            let _ = self.app.emit("run-event", event);
        }
    }

    fn state(&self, run_id: i64, state: &str, summary: Option<&str>) {
        let store = self.app.state::<Database>();
        let summary = summary.map(redact::clean);
        let updated = store.connection().and_then(|db| {
            run::set_state(&db, run_id, state, summary.as_deref())?;
            run::get(&db, run_id)
        });
        if let Ok(run) = updated {
            if !self.read_only {
                sync_task(&self.app, &run);
            }
            let _ = self.app.emit("run-state", run);
        }
    }

    fn blocked(&self, run_id: i64, reason: &str) {
        let reason = redact::clean(reason);
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::set_blocked(&db, run_id, &reason));
    }

    fn proof(&self, run_id: i64, verdict: &str) {
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::set_proof(&db, run_id, verdict));
    }

    fn worktree(&self, run_id: i64, base_sha: &str, branch: &str, path: &str) {
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::set_worktree(&db, run_id, base_sha, branch, path));
    }

    fn cycle(&self, run_id: i64, cycle: &CycleRecord<'_>) {
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::record_cycle(&db, run_id, cycle));
    }

    fn review(&self, run_id: i64, verdict: &str, discarded: usize) {
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::set_review(&db, run_id, verdict, discarded));
    }

    fn denials(&self, run_id: i64, denied: &[String]) {
        let store = self.app.state::<Database>();
        let _ = store.connection().and_then(|db| {
            let run = run::get(&db, run_id)?;
            run::record_denials(&db, run.workspace_id, denied)
        });
    }

    fn checks_ms(&self, run_id: i64, ms: i64) {
        let store = self.app.state::<Database>();
        let _ = store
            .connection()
            .and_then(|db| run::add_checks_ms(&db, run_id, ms));
    }
}

/// Espelha o estado da run no card (regras em [`sync::reconcile`]); falha de
/// banco não derruba a execução.
fn sync_task(app: &AppHandle, run: &Run) {
    let store = app.state::<Database>();
    let change = store.connection().and_then(|db| sync::reconcile(&db, run));
    if let Ok(Some(change)) = change {
        emit_change(app, &change);
    }
}

/// Binários e pasta para falar com o GitHub em nome de uma run; falha com a frase certa.
fn delivery_tools(
    store: &Database,
    run_id: i64,
) -> Result<(PathBuf, PathBuf, Vec<PathBuf>, PathBuf)> {
    let folder = {
        let db = store.connection()?;
        let run = run::get(&db, run_id)?;
        workspace::get(&db, run.workspace_id)?.folder
    };
    process::wait_for_login_path();
    let dirs = process::search_dirs();
    let git = process::find_binary("git", &dirs)
        .ok_or(AppError::Validation("git não encontrado neste computador."))?;
    let gh = process::find_binary("gh", &dirs).ok_or(AppError::Validation(
        "GitHub CLI (gh) não encontrado. Veja Conexões.",
    ))?;
    Ok((git, gh, dirs, PathBuf::from(folder)))
}

/// Registra na conversa da run o que o Harnessy fez no GitHub.
fn note_delivery(app: &AppHandle, run_id: i64, text: &str) {
    let store = app.state::<Database>();
    let _ = store
        .connection()
        .and_then(|db| run::append_event(&db, run_id, "status", text));
}

/// Envia a branch da run e abre o PR em draft (ou adota o que já existe).
#[tauri::command]
pub async fn publish_run(
    app: AppHandle,
    store: State<'_, Database>,
    run_id: i64,
) -> Result<crate::delivery::Delivery> {
    let (git, gh, dirs, repo) = delivery_tools(&store, run_id)?;
    let tools = run::publish::Tools {
        git: &git,
        gh: &gh,
        dirs: &dirs,
        repo: &repo,
    };
    let result = super::delivery::publish(&store, &tools, &git, run_id)?;
    note_delivery(
        &app,
        run_id,
        &format!(
            "PR #{} publicado como draft: {}",
            result.pr_number, result.pr_url
        ),
    );
    Ok(result)
}

/// Relê o PR no GitHub (checks, conflitos, merge feito por fora).
#[tauri::command]
pub async fn refresh_delivery(
    app: AppHandle,
    store: State<'_, Database>,
    run_id: i64,
) -> Result<crate::delivery::Delivery> {
    let (git, gh, dirs, repo) = delivery_tools(&store, run_id)?;
    let tools = run::publish::Tools {
        git: &git,
        gh: &gh,
        dirs: &dirs,
        repo: &repo,
    };
    let (result, change) = super::delivery::refresh(&store, &tools, run_id)?;
    if let Some(change) = change {
        emit_change(&app, &change);
    }
    Ok(result)
}

/// Merge (squash) do PR da run, só com o PR seguro; a tarefa vai para Concluído.
#[tauri::command]
pub async fn merge_delivery(
    app: AppHandle,
    store: State<'_, Database>,
    run_id: i64,
) -> Result<crate::delivery::Delivery> {
    let (git, gh, dirs, repo) = delivery_tools(&store, run_id)?;
    let tools = run::publish::Tools {
        git: &git,
        gh: &gh,
        dirs: &dirs,
        repo: &repo,
    };
    let (result, change) = super::delivery::merge(&store, &tools, run_id)?;
    note_delivery(&app, run_id, &format!("PR #{} mergeado.", result.pr_number));
    if let Some(change) = change {
        emit_change(&app, &change);
    }
    Ok(result)
}

#[tauri::command]
pub fn list_deliveries(
    store: State<'_, Database>,
    workspace_id: i64,
) -> Result<Vec<crate::delivery::Delivery>> {
    super::delivery::list(&*store.connection()?, workspace_id)
}

/// Como subir o projeto: o perfil do workspace, se foi confirmado e o que sugerir.
#[tauri::command]
pub fn get_profile(
    store: State<'_, Database>,
    workspace_id: i64,
) -> Result<super::preview::ProfileView> {
    super::preview::profile_view(&*store.connection()?, workspace_id)
}

/// Grava `.harnessy/workspace.toml` com o comando escolhido e o confirma.
#[tauri::command]
pub fn save_profile(
    store: State<'_, Database>,
    workspace_id: i64,
    run: String,
    port: Option<u16>,
) -> Result<super::preview::ProfileView> {
    super::preview::save_profile(&*store.connection()?, workspace_id, &run, port)
}

/// Confirma o comando que o repositório já traz no perfil.
#[tauri::command]
pub fn approve_profile(
    store: State<'_, Database>,
    workspace_id: i64,
) -> Result<super::preview::ProfileView> {
    super::preview::approve_profile(&*store.connection()?, workspace_id)
}

fn emit_preview(app: &AppHandle, event: run::preview::Event) {
    match event {
        run::preview::Event::State(info) => {
            let _ = app.emit("preview-state", info);
        }
        run::preview::Event::Log { run_id, line } => {
            let _ = app.emit(
                "preview-log",
                serde_json::json!({ "runId": run_id, "line": line }),
            );
        }
    }
}

/// Sobe o projeto da run no worktree dela, com o comando que o usuário confirmou.
#[tauri::command]
pub fn start_preview(
    app: AppHandle,
    store: State<'_, Database>,
    previews: State<'_, run::preview::Previews>,
    run_id: i64,
) -> Result<run::preview::Info> {
    process::wait_for_login_path();
    let dirs = process::search_dirs();
    let target = app_data(&app)?.join("cargo-target");
    let spec = super::preview::spec_for(&*store.connection()?, run_id, &dirs, &target)?;
    let handle = app.clone();
    previews.start(spec, move |event| emit_preview(&handle, event))
}

#[tauri::command]
pub fn stop_preview(previews: State<'_, run::preview::Previews>, run_id: i64) -> Result<()> {
    previews.stop(run_id);
    Ok(())
}

/// Abre no navegador o endereço local que o ambiente informou.
#[tauri::command]
pub fn open_preview(previews: State<'_, run::preview::Previews>, run_id: i64) -> Result<()> {
    previews.open(run_id)
}

/// Estado e últimas linhas do log do ambiente de uma run (a tela chega depois dos eventos).
#[tauri::command]
pub fn preview_snapshot(
    previews: State<'_, run::preview::Previews>,
    run_id: i64,
) -> Result<(Option<run::preview::Info>, Vec<String>)> {
    Ok((previews.info(run_id), previews.log(run_id)))
}

/// Ambientes de teste vivos, para o quadro marcar quais tarefas estão no ar.
#[tauri::command]
pub fn list_previews(
    previews: State<'_, run::preview::Previews>,
) -> Result<Vec<run::preview::Info>> {
    Ok(previews.active())
}
