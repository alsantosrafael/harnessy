//! Automação do ciclo: em quais etapas o humano conduz, aprova ou delega ao agente.
//!
//! É uma escolha **pessoal**, guardada no banco local e nunca no repositório: um
//! projeto clonado não pode se autorizar a publicar nem mesclar sozinho. O
//! repositório só pode apertar as travas (caminhos sensíveis, teto do diff), nunca
//! afrouxá-las. A decisão em si é pura e fica em [`decide`].

// Temporário: `decide`, `record` e `tightened` ganham chamador no motor de etapas (flow), na 2b-2.
#![allow(dead_code)]

pub mod commands;
pub mod decide;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    db::now,
    error::{AppError, Result},
};

const DEFAULT_MAX_DIFF_LINES: u32 = 400;
const DEFAULT_MAX_FILES: u32 = 12;
const MAX_ALLOWED_DIFF_LINES: u32 = 5_000;
const MAX_ALLOWED_FILES: u32 = 200;
const MAX_SENSITIVE: usize = 40;
const MAX_SENSITIVE_CHARS: usize = 120;
const DEFAULT_SENSITIVE: [&str; 7] = [
    "src-tauri/migrations",
    "package.json",
    "bun.lock",
    "Cargo.toml",
    "Cargo.lock",
    ".github/",
    ".harnessy/",
];
const MAX_EVENT_REASON: usize = 300;

/// Etapas do ciclo, na ordem em que acontecem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Começar a implementar a tarefa.
    Start,
    /// Revisão do staff engineer sobre o diff.
    Review,
    /// QA: casos de uso, bordas e regressões.
    Qa,
    /// Subir o ambiente de teste local.
    Preview,
    /// Enviar a branch e abrir o PR em draft.
    Publish,
    /// Mesclar o PR.
    Merge,
}

impl Stage {
    pub const ALL: [Stage; 6] = [
        Stage::Start,
        Stage::Review,
        Stage::Qa,
        Stage::Preview,
        Stage::Publish,
        Stage::Merge,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Stage::Start => "start",
            Stage::Review => "review",
            Stage::Qa => "qa",
            Stage::Preview => "preview",
            Stage::Publish => "publish",
            Stage::Merge => "merge",
        }
    }
}

/// Quem faz o quê numa etapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nada acontece sozinho; a interface mostra a próxima ação.
    Manual,
    /// O Harnessy prepara tudo e pede só a confirmação, com prévia.
    Assisted,
    /// Executa e avisa. Só roda se as travas passarem.
    Auto,
}

impl Mode {
    fn id(self) -> &'static str {
        match self {
            Mode::Manual => "manual",
            Mode::Assisted => "assisted",
            Mode::Auto => "auto",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "manual" => Some(Mode::Manual),
            "assisted" => Some(Mode::Assisted),
            "auto" => Some(Mode::Auto),
            _ => None,
        }
    }
}

/// Um modo por etapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Modes {
    pub start: Mode,
    pub review: Mode,
    pub qa: Mode,
    pub preview: Mode,
    pub publish: Mode,
    pub merge: Mode,
}

impl Modes {
    pub fn get(&self, stage: Stage) -> Mode {
        match stage {
            Stage::Start => self.start,
            Stage::Review => self.review,
            Stage::Qa => self.qa,
            Stage::Preview => self.preview,
            Stage::Publish => self.publish,
            Stage::Merge => self.merge,
        }
    }

    fn set(&mut self, stage: Stage, mode: Mode) {
        match stage {
            Stage::Start => self.start = mode,
            Stage::Review => self.review = mode,
            Stage::Qa => self.qa = mode,
            Stage::Preview => self.preview = mode,
            Stage::Publish => self.publish = mode,
            Stage::Merge => self.merge = mode,
        }
    }

    /// O merge nunca é automático nesta versão: o máximo é pedir o OK.
    fn validate(&self) -> Result<()> {
        if self.merge == Mode::Auto {
            return Err(AppError::Validation(
                "O merge automático ainda não existe: use Assistido para receber o merge pronto.",
            ));
        }
        Ok(())
    }
}

/// Atalhos de intenção; cada um é só um conjunto de modos.
pub fn preset(id: &str) -> Option<Modes> {
    use Mode::{Assisted, Auto, Manual};
    let modes = |start, review, qa, preview, publish, merge| Modes {
        start,
        review,
        qa,
        preview,
        publish,
        merge,
    };
    match id {
        "conduzo" => Some(modes(Manual, Manual, Manual, Manual, Manual, Manual)),
        "copiloto" => Some(modes(Manual, Auto, Auto, Auto, Assisted, Manual)),
        "cinto" => Some(modes(Assisted, Auto, Auto, Auto, Auto, Assisted)),
        _ => None,
    }
}

pub const DEFAULT_PRESET: &str = "copiloto";

/// Travas que valem para qualquer etapa delegada. O repositório pode apertá-las.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Guardrails {
    pub max_diff_lines: u32,
    pub max_files: u32,
    /// Prefixos de caminho cuja alteração tira a etapa do automático.
    pub sensitive: Vec<String>,
}

impl Default for Guardrails {
    fn default() -> Self {
        Self {
            max_diff_lines: DEFAULT_MAX_DIFF_LINES,
            max_files: DEFAULT_MAX_FILES,
            sensitive: DEFAULT_SENSITIVE
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
        }
    }
}

impl Guardrails {
    fn validate(&self) -> Result<()> {
        if self.max_diff_lines == 0 || self.max_diff_lines > MAX_ALLOWED_DIFF_LINES {
            return Err(AppError::Validation(
                "O teto de linhas do diff deve ficar entre 1 e 5.000.",
            ));
        }
        if self.max_files == 0 || self.max_files > MAX_ALLOWED_FILES {
            return Err(AppError::Validation(
                "O teto de arquivos deve ficar entre 1 e 200.",
            ));
        }
        if self.sensitive.len() > MAX_SENSITIVE
            || self.sensitive.iter().any(|path| {
                path.trim().is_empty()
                    || path.chars().count() > MAX_SENSITIVE_CHARS
                    || path.contains(['\n', '\r'])
                    || path.starts_with('/')
                    || path.contains("..")
            })
        {
            return Err(AppError::Validation(
                "Os caminhos sensíveis devem ser relativos, de uma linha, até 40 itens.",
            ));
        }
        Ok(())
    }

    /// Junta o que o repositório pede: só aperta (teto menor, mais caminhos), nunca afrouxa.
    pub fn tightened(&self, repo_max_lines: Option<u32>, repo_sensitive: &[String]) -> Self {
        let mut next = self.clone();
        if let Some(lines) = repo_max_lines.filter(|lines| *lines > 0) {
            next.max_diff_lines = next.max_diff_lines.min(lines);
        }
        for path in repo_sensitive {
            if !next.sensitive.contains(path) {
                next.sensitive.push(path.clone());
            }
        }
        next
    }
}

/// Configuração do workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub modes: Modes,
    pub guardrails: Guardrails,
    /// Pausa geral: nenhuma etapa delegada roda enquanto estiver ligada.
    pub paused: bool,
}

impl Settings {
    fn default_for_new() -> Self {
        Self {
            modes: preset(DEFAULT_PRESET).unwrap_or(Modes {
                start: Mode::Manual,
                review: Mode::Manual,
                qa: Mode::Manual,
                preview: Mode::Manual,
                publish: Mode::Manual,
                merge: Mode::Manual,
            }),
            guardrails: Guardrails::default(),
            paused: false,
        }
    }
}

const SETTINGS_COLUMNS: &str =
    "start, review, qa, preview, publish, merge, paused, max_diff_lines, max_files, sensitive";

fn mode_at(r: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Mode> {
    let text: String = r.get(index)?;
    Mode::parse(&text).ok_or(rusqlite::Error::InvalidQuery)
}

pub fn settings(db: &Connection, workspace_id: i64) -> Result<Settings> {
    let found = db
        .query_row(
            &format!("SELECT {SETTINGS_COLUMNS} FROM workspace_automation WHERE workspace_id=?1"),
            [workspace_id],
            |r| {
                let sensitive: String = r.get(9)?;
                Ok(Settings {
                    modes: Modes {
                        start: mode_at(r, 0)?,
                        review: mode_at(r, 1)?,
                        qa: mode_at(r, 2)?,
                        preview: mode_at(r, 3)?,
                        publish: mode_at(r, 4)?,
                        merge: mode_at(r, 5)?,
                    },
                    paused: r.get::<_, i64>(6)? != 0,
                    guardrails: Guardrails {
                        max_diff_lines: r.get(7)?,
                        max_files: r.get(8)?,
                        sensitive: sensitive
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .map(str::to_owned)
                            .collect(),
                    },
                })
            },
        )
        .optional()?;
    Ok(found.unwrap_or_else(Settings::default_for_new))
}

/// Grava modos e travas do workspace (validados). A pausa tem comando próprio.
pub fn save_settings(
    db: &Connection,
    workspace_id: i64,
    modes: &Modes,
    guardrails: &Guardrails,
) -> Result<Settings> {
    modes.validate()?;
    guardrails.validate()?;
    db.execute(
        "INSERT INTO workspace_automation (workspace_id, start, review, qa, preview, publish, merge, max_diff_lines, max_files, sensitive) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
         ON CONFLICT(workspace_id) DO UPDATE SET start=?2, review=?3, qa=?4, preview=?5, publish=?6, merge=?7, \
         max_diff_lines=?8, max_files=?9, sensitive=?10",
        params![
            workspace_id,
            modes.start.id(),
            modes.review.id(),
            modes.qa.id(),
            modes.preview.id(),
            modes.publish.id(),
            modes.merge.id(),
            guardrails.max_diff_lines,
            guardrails.max_files,
            guardrails.sensitive.join("\n"),
        ],
    )?;
    settings(db, workspace_id)
}

/// Liga ou desliga a pausa geral, mantendo o restante.
pub fn set_paused(db: &Connection, workspace_id: i64, paused: bool) -> Result<Settings> {
    let current = settings(db, workspace_id)?;
    save_settings(db, workspace_id, &current.modes, &current.guardrails)?;
    db.execute(
        "UPDATE workspace_automation SET paused=?2 WHERE workspace_id=?1",
        params![workspace_id, paused],
    )?;
    settings(db, workspace_id)
}

/// Exceções de uma tarefa: etapas em que o humano quis outro modo que o do workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Overrides {
    pub start: Option<Mode>,
    pub review: Option<Mode>,
    pub qa: Option<Mode>,
    pub preview: Option<Mode>,
    pub publish: Option<Mode>,
    pub merge: Option<Mode>,
}

impl Overrides {
    fn get(&self, stage: Stage) -> Option<Mode> {
        match stage {
            Stage::Start => self.start,
            Stage::Review => self.review,
            Stage::Qa => self.qa,
            Stage::Preview => self.preview,
            Stage::Publish => self.publish,
            Stage::Merge => self.merge,
        }
    }

    /// Modos efetivos: a exceção da tarefa vence o padrão do workspace.
    pub fn apply(&self, base: &Modes) -> Modes {
        let mut modes = *base;
        for stage in Stage::ALL {
            if let Some(mode) = self.get(stage) {
                modes.set(stage, mode);
            }
        }
        modes
    }
}

pub fn overrides(db: &Connection, task_id: i64) -> Result<Overrides> {
    let found = db
        .query_row(
            "SELECT start, review, qa, preview, publish, merge FROM task_automation WHERE task_id=?1",
            [task_id],
            |r| {
                let at = |i: usize| -> rusqlite::Result<Option<Mode>> {
                    Ok(r.get::<_, Option<String>>(i)?.and_then(|text| Mode::parse(&text)))
                };
                Ok(Overrides {
                    start: at(0)?,
                    review: at(1)?,
                    qa: at(2)?,
                    preview: at(3)?,
                    publish: at(4)?,
                    merge: at(5)?,
                })
            },
        )
        .optional()?;
    Ok(found.unwrap_or_default())
}

/// Grava as exceções da tarefa; tudo `None` apaga a linha (volta ao padrão do workspace).
pub fn save_overrides(db: &Connection, task_id: i64, value: &Overrides) -> Result<Overrides> {
    if value.merge == Some(Mode::Auto) {
        return Err(AppError::Validation(
            "O merge automático ainda não existe: use Assistido para receber o merge pronto.",
        ));
    }
    if *value == Overrides::default() {
        db.execute("DELETE FROM task_automation WHERE task_id=?1", [task_id])?;
        return Ok(*value);
    }
    let text = |mode: Option<Mode>| mode.map(Mode::id);
    db.execute(
        "INSERT INTO task_automation (task_id, start, review, qa, preview, publish, merge) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT(task_id) DO UPDATE SET start=?2, review=?3, qa=?4, preview=?5, publish=?6, merge=?7",
        params![
            task_id,
            text(value.start),
            text(value.review),
            text(value.qa),
            text(value.preview),
            text(value.publish),
            text(value.merge)
        ],
    )?;
    overrides(db, task_id)
}

/// Atalho oferecido na interface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub id: &'static str,
    pub modes: Modes,
}

/// Tudo o que a tela de automação precisa de uma vez.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub settings: Settings,
    /// Exceções da tarefa; vazio quando nenhuma tarefa foi pedida.
    pub overrides: Overrides,
    /// Modos que valem de fato para a tarefa (exceções sobre o padrão do workspace).
    pub effective: Modes,
    pub presets: Vec<PresetView>,
}

pub fn view(db: &Connection, workspace_id: i64, task_id: Option<i64>) -> Result<View> {
    let settings = settings(db, workspace_id)?;
    let overrides = match task_id {
        Some(id) => overrides(db, id)?,
        None => Overrides::default(),
    };
    let effective = overrides.apply(&settings.modes);
    let presets = ["conduzo", "copiloto", "cinto"]
        .into_iter()
        .filter_map(|id| preset(id).map(|modes| PresetView { id, modes }))
        .collect();
    Ok(View {
        settings,
        overrides,
        effective,
        presets,
    })
}

/// Aplica um atalho aos modos do workspace, mantendo travas e pausa.
pub fn apply_preset(db: &Connection, workspace_id: i64, id: &str) -> Result<Settings> {
    let modes = preset(id).ok_or(AppError::Validation("Atalho de automação desconhecido."))?;
    let current = settings(db, workspace_id)?;
    save_settings(db, workspace_id, &modes, &current.guardrails)
}

/// O que o Harnessy decidiu numa etapa, para o registro que o humano consulta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Executou sozinho.
    Ran,
    /// Preparou e ficou esperando o OK.
    Asked,
    /// Ainda não dá: falta algo antes (por exemplo, a run terminar).
    Waiting,
    /// Parou e entregou ao humano por causa de uma trava.
    Held,
}

impl Outcome {
    fn id(self) -> &'static str {
        match self {
            Outcome::Ran => "ran",
            Outcome::Asked => "asked",
            Outcome::Waiting => "waiting",
            Outcome::Held => "held",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: i64,
    pub task_id: i64,
    pub run_id: Option<i64>,
    pub stage: String,
    pub outcome: String,
    pub reason: String,
    pub at: i64,
}

/// Registra uma decisão. Mesma etapa, mesmo desfecho e mesmo motivo seguidos não se repetem.
pub fn record(
    db: &Connection,
    workspace_id: i64,
    task_id: i64,
    run_id: Option<i64>,
    stage: Stage,
    outcome: Outcome,
    reason: &str,
) -> Result<bool> {
    let reason: String = reason.chars().take(MAX_EVENT_REASON).collect();
    let last: Option<(String, String)> = db
        .query_row(
            "SELECT outcome, reason FROM automation_events WHERE task_id=?1 AND stage=?2 ORDER BY id DESC LIMIT 1",
            params![task_id, stage.id()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if last.is_some_and(|(o, r)| o == outcome.id() && r == reason) {
        return Ok(false);
    }
    db.execute(
        "INSERT INTO automation_events (workspace_id, task_id, run_id, stage, outcome, reason, at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![workspace_id, task_id, run_id, stage.id(), outcome.id(), reason, now()],
    )?;
    Ok(true)
}

pub fn events(
    db: &Connection,
    workspace_id: i64,
    task_id: Option<i64>,
    limit: u32,
) -> Result<Vec<Event>> {
    let mut query = db.prepare(
        "SELECT id, task_id, run_id, stage, outcome, reason, at FROM automation_events \
         WHERE workspace_id=?1 AND (?2 IS NULL OR task_id=?2) ORDER BY id DESC LIMIT ?3",
    )?;
    let rows = query.query_map(params![workspace_id, task_id, limit.min(200)], |r| {
        Ok(Event {
            id: r.get(0)?,
            task_id: r.get(1)?,
            run_id: r.get(2)?,
            stage: r.get(3)?,
            outcome: r.get(4)?,
            reason: r.get(5)?,
            at: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::TempDatabase;

    fn with_db(label: &str, f: impl FnOnce(&Connection)) {
        let temp = TempDatabase::new(label);
        let store = temp.open();
        let db = store.connection().unwrap();
        db.execute("PRAGMA foreign_keys=OFF", []).unwrap();
        f(&db);
    }

    #[test]
    fn presets_never_delegate_the_merge() {
        for id in ["conduzo", "copiloto", "cinto"] {
            let modes = preset(id).unwrap();
            assert!(modes.validate().is_ok(), "{id}");
            assert_ne!(modes.merge, Mode::Auto, "{id}");
        }
        assert_eq!(preset("copiloto").unwrap().publish, Mode::Assisted);
        assert!(preset("inexistente").is_none());
    }

    #[test]
    fn a_new_workspace_starts_as_copilot_and_settings_round_trip() {
        with_db("auto-settings", |db| {
            assert_eq!(settings(db, 1).unwrap().modes, preset("copiloto").unwrap());
            let mut modes = preset("conduzo").unwrap();
            modes.preview = Mode::Auto;
            let rails = Guardrails {
                max_diff_lines: 100,
                ..Guardrails::default()
            };
            let saved = save_settings(db, 1, &modes, &rails).unwrap();
            assert_eq!(saved.modes.preview, Mode::Auto);
            assert_eq!(saved.guardrails.max_diff_lines, 100);
            assert!(!saved.paused);
            assert!(set_paused(db, 1, true).unwrap().paused);
            assert_eq!(
                settings(db, 1).unwrap().modes.preview,
                Mode::Auto,
                "pausar não mexe nos modos"
            );
        });
    }

    #[test]
    fn invalid_settings_are_refused() {
        with_db("auto-invalid", |db| {
            let mut modes = preset("copiloto").unwrap();
            modes.merge = Mode::Auto;
            assert!(save_settings(db, 1, &modes, &Guardrails::default()).is_err());
            let bad = |rails: Guardrails| save_settings(db, 1, &preset("conduzo").unwrap(), &rails);
            assert!(bad(Guardrails {
                max_diff_lines: 0,
                ..Guardrails::default()
            })
            .is_err());
            assert!(bad(Guardrails {
                max_files: 999,
                ..Guardrails::default()
            })
            .is_err());
            assert!(bad(Guardrails {
                sensitive: vec!["/etc".into()],
                ..Guardrails::default()
            })
            .is_err());
            assert!(bad(Guardrails {
                sensitive: vec!["a/../b".into()],
                ..Guardrails::default()
            })
            .is_err());
        });
    }

    #[test]
    fn task_overrides_win_and_clearing_them_returns_to_the_workspace() {
        with_db("auto-overrides", |db| {
            let base = preset("copiloto").unwrap();
            let careful = Overrides {
                preview: Some(Mode::Manual),
                publish: Some(Mode::Manual),
                ..Overrides::default()
            };
            save_overrides(db, 7, &careful).unwrap();
            let effective = overrides(db, 7).unwrap().apply(&base);
            assert_eq!(
                (effective.preview, effective.publish, effective.review),
                (Mode::Manual, Mode::Manual, Mode::Auto)
            );
            save_overrides(db, 7, &Overrides::default()).unwrap();
            assert_eq!(overrides(db, 7).unwrap().apply(&base), base);
            assert!(save_overrides(
                db,
                7,
                &Overrides {
                    merge: Some(Mode::Auto),
                    ..Overrides::default()
                }
            )
            .is_err());
        });
    }

    #[test]
    fn the_repository_can_tighten_the_rails_but_never_loosen_them() {
        let rails = Guardrails::default();
        let tight = rails.tightened(Some(50), &["migrations/".into(), "package.json".into()]);
        assert_eq!(tight.max_diff_lines, 50);
        assert!(tight.sensitive.contains(&"migrations/".to_owned()));
        assert_eq!(
            tight
                .sensitive
                .iter()
                .filter(|p| *p == "package.json")
                .count(),
            1
        );
        let loose = rails.tightened(Some(99_999), &[]);
        assert_eq!(
            loose.max_diff_lines, DEFAULT_MAX_DIFF_LINES,
            "um teto maior que o do usuário é ignorado"
        );
    }

    #[test]
    fn the_log_does_not_repeat_the_same_decision() {
        with_db("auto-events", |db| {
            let log = |outcome, reason| {
                record(db, 1, 5, Some(9), Stage::Publish, outcome, reason).unwrap()
            };
            assert!(log(Outcome::Held, "diff grande"));
            assert!(!log(Outcome::Held, "diff grande"), "repetição seguida");
            assert!(log(Outcome::Ran, ""), "outro desfecho entra");
            assert!(
                log(Outcome::Held, "diff grande"),
                "voltar a um desfecho antigo entra"
            );
            let all = events(db, 1, None, 10).unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].outcome, "held");
            assert_eq!(events(db, 1, Some(99), 10).unwrap().len(), 0);
        });
    }
}
