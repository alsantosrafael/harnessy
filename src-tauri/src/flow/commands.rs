//! Adaptadores IPC que cruzam domínios: prévia do prompt e início de uma run.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter, Manager, State};

use super::{
    actions, continue_with, execute, likely, read, review::Reviewer, sync, verify, Job, Sink, ROLE,
    RUNTIME,
};
use crate::{
    action::{self, Acesso, Alvo, Contexto, Quando, Repositorio, Saida},
    db::Database,
    error::{AppError, Result},
    process,
    run::{
        self,
        claude::ClaudeCode,
        codex::{self, Codex},
        guard, redact,
        runtime::Runtime,
        supervisor::Registry,
        CycleRecord, NewRun, Run,
    },
    task::{self, commands::emit_change, Task},
    workspace,
};

const MAX_TITLE_BYTES: usize = 300;

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

/// Revisor da run de escrita: a Ação `revisar` do catálogo (a do usuário vence),
/// salvo quando ela está inválida, não é sobre Run ou está em `quando = "manual"`.
/// Com `codex`, a revisão roda no Codex; sem ele, no Claude Code, um degrau acima.
fn reviewer_for(
    data_dir: &Path,
    repo: Option<&Repositorio<'_>>,
    task: &Task,
    model: &str,
    codex: Option<Codex>,
) -> Option<Reviewer> {
    let acao = actions::find(data_dir, repo, action::REVIEW).ok()?;
    if acao.alvo != Alvo::Run || acao.saida != Saida::Revisao || acao.quando == Quando::Manual {
        return None;
    }
    let (model, runtime) = match codex {
        Some(codex) => (
            codex::LABEL.to_owned(),
            Some(Box::new(codex) as Box<dyn Runtime + Send>),
        ),
        None => (Reviewer::model_for(&acao, model), None),
    };
    Some(Reviewer {
        model,
        runtime,
        action: acao,
        tarefa: task.title.clone(),
        descricao: task.description.clone(),
        criterios: task.criteria.clone(),
    })
}

/// Codex do revisor escolhido; `None` quando o revisor é o Claude Code.
fn codex_for(reviewer: &str, dirs: &[PathBuf], data_dir: &Path) -> Result<Option<Codex>> {
    if reviewer != "codex" {
        return Ok(None);
    }
    let binary = process::find_binary("codex", dirs).ok_or(AppError::Validation(
        "Codex não encontrado neste computador.",
    ))?;
    let env = guard::env(dirs, &data_dir.join("cargo-target"), |name| {
        std::env::var(name).ok()
    });
    Ok(Some(Codex::new(binary, data_dir.to_path_buf(), env)))
}

/// Prompt de uma Ação sobre uma Tarefa: arquivos prováveis da pasta do workspace
/// e comandos que o Harnessy já negou ali.
fn prompt_for(acao: &action::Action, task: &Task, folder: &str, avoid: &[String]) -> String {
    let arquivos = likely::find(Path::new(folder), &task.title, &task.criteria);
    actions::build_prompt(acao, &contexto(task, &arquivos), avoid)
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
) -> Result<Run> {
    if runtime != RUNTIME {
        return Err(AppError::Validation(
            "Este agente ainda não tem adaptador de execução. Use o Claude Code.",
        ));
    }
    let model = run::guard::model(&model)?;
    let reviewer = guard::reviewer(&reviewer)?;
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
    let claude_bin = process::find_binary("claude", &dirs).ok_or(AppError::Validation(
        "Claude Code não encontrado neste computador.",
    ))?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Run("pasta de dados indisponível.".into()))?;
    // Só run de escrita é revisada; o Codex ausente recusa antes de criar a run.
    let codex = if read_only {
        None
    } else {
        codex_for(reviewer, &dirs, &data_dir)?
    };
    let cargo_target = data_dir.join("cargo-target");
    let runtime = ClaudeCode::new(
        claude_bin,
        data_dir.clone(),
        guard::env(&dirs, &cargo_target, |name| std::env::var(name).ok()),
    );

    let title: String = task.title.chars().take(MAX_TITLE_BYTES / 4).collect();
    let created = run::create(
        &*store.connection()?,
        &NewRun {
            workspace_id,
            task_id: task.id,
            title: &title,
            action: &acao.id,
            role: ROLE,
            runtime: RUNTIME,
            model,
            effort: acao.esforco.as_deref(),
            base_run_id: base.as_ref().map(|(id, _)| *id),
        },
    )?;
    if !read_only {
        run::set_reviewer(&*store.connection()?, created.id, reviewer)?;
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
    let job = Job {
        run_id: created.id,
        title,
        prompt: prompt_for(&acao, &task, &ws.folder, &avoid),
        repo: ws.folder.into(),
        data_dir,
        model: model.into(),
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
        review: if read_only {
            None
        } else {
            reviewer_for(&data_dir_review, Some(&repo), &task, model, codex)
        },
        base_ref: base.map(|(_, branch)| branch),
        output_schema: (acao.saida == Saida::Passos).then_some(read::STEPS_SCHEMA),
    };
    let handle = app.clone();
    let sink = AppSink {
        app: app.clone(),
        read_only,
    };
    std::thread::spawn(move || {
        let registry = handle.state::<Registry>();
        if read_only {
            read::execute(job, &registry, &sink, &runtime);
        } else {
            execute(job, &registry, &sink, &runtime);
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
    let claude_bin = process::find_binary("claude", &dirs).ok_or(AppError::Validation(
        "Claude Code não encontrado neste computador.",
    ))?;
    let data_dir = app_data(&app)?;
    let codex = codex_for(
        guard::reviewer(previous.reviewer.as_deref().unwrap_or("claude"))?,
        &dirs,
        &data_dir,
    )?;
    let runtime = ClaudeCode::new(
        claude_bin,
        data_dir.clone(),
        guard::env(&dirs, &data_dir.join("cargo-target"), |name| {
            std::env::var(name).ok()
        }),
    );
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
        repo: ws.folder.into(),
        data_dir,
        model: model.into(),
        checks: verify::checks_for(&dir),
        max_corrections: 0,
        contraprova: acao.contraprova,
        require_proof: acao.exigir_prova,
        effort: acao.esforco.clone(),
        max_turns: actions::max_turns(&acao),
        review: reviewer_for(&app_data(&app)?, Some(&repo), &task, model, codex),
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
        continue_with(job, dir, base, offset, &registry, &sink, &runtime);
    });
    Ok(reopened)
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
