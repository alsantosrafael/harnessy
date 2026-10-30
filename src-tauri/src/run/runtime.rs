//! Contrato neutro com o runtime de agente (Claude Code hoje, outros depois).
//!
//! O pipeline de `flow` depende só deste módulo. Quem conhece a CLI de verdade
//! é o adaptador (`run::claude`), que traduz o protocolo dela em [`AgentEvent`]
//! e devolve um [`Exit`]. Desconhecido é sempre `None`, nunca zero.

use std::path::Path;

use super::supervisor::Registry;

/// Nível de acesso do agente. A Ação escolhe o nível; o adaptador traduz para ferramentas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

/// O que o runtime precisa para iniciar um ciclo do agente.
pub struct AgentSpec<'a> {
    pub dir: &'a Path,
    pub prompt: &'a str,
    pub model: &'a str,
    pub max_turns: u32,
    pub access: Access,
    /// Esforço de raciocínio; ignorado por runtime que não o suporta.
    pub effort: Option<&'a str>,
    /// Sessão do ciclo anterior a retomar; só usada quando [`Runtime::resumes`].
    pub resume: Option<&'a str>,
    /// JSON Schema da saída estruturada; a saída chega em [`Finish::structured`].
    pub json_schema: Option<&'a str>,
}

impl<'a> AgentSpec<'a> {
    /// Ciclo sem esforço, retomada nem saída estruturada.
    pub fn new(
        dir: &'a Path,
        prompt: &'a str,
        model: &'a str,
        max_turns: u32,
        access: Access,
    ) -> Self {
        Self {
            dir,
            prompt,
            model,
            max_turns,
            access,
            effort: None,
            resume: None,
            json_schema: None,
        }
    }
}

/// Identidade da run no supervisor: cancelamento e registro do processo.
pub struct Session<'a> {
    pub run_id: i64,
    pub registry: &'a Registry,
}

/// Uso de um ciclo. Cada campo é `None` quando o runtime não informou.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CycleUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub cost_usd: Option<f64>,
    pub turns: Option<u64>,
}

/// Fim de um ciclo reportado pelo runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct Finish {
    pub ok: bool,
    pub subtype: String,
    pub summary: String,
    pub session_id: Option<String>,
    pub usage: CycleUsage,
    /// Ações negadas pelo runtime neste ciclo (todas, não só as detalhadas).
    pub denials: usize,
    /// Saída estruturada (JSON) quando o ciclo pediu `json_schema`.
    pub structured: Option<String>,
}

impl Finish {
    /// Linha exibida no log da run.
    pub fn line(&self) -> String {
        let turns = self
            .usage
            .turns
            .map_or("turnos não informados".into(), |t| format!("{t} turnos"));
        let cost = self
            .usage
            .cost_usd
            .map_or("custo não informado".into(), |c| format!("US$ {c:.2}"));
        let mut line = format!("{} · {turns} · {cost}", self.subtype);
        if self.denials > 0 {
            line.push_str(&format!(" · {} ações negadas", self.denials));
        }
        line
    }
}

/// Evento neutro vindo de um ciclo do agente.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    /// Sessão do runtime iniciada (usada para retomada em passos futuros).
    Started {
        session_id: String,
    },
    Text(String),
    Tool(String),
    Denied(String),
    Finished(Finish),
    /// Linha de saída que não é evento do protocolo.
    Log(String),
    Stderr(String),
}

impl AgentEvent {
    /// Tipo e texto gravados no log da run; `None` quando o evento não aparece no log.
    pub fn log_line(&self) -> Option<(&'static str, String)> {
        match self {
            Self::Started { .. } => None,
            Self::Text(text) => Some(("assistant", text.clone())),
            Self::Tool(text) => Some(("tool", text.clone())),
            Self::Denied(text) => Some(("denied", text.clone())),
            Self::Finished(finish) => Some(("result", finish.line())),
            Self::Log(text) => Some(("log", text.clone())),
            Self::Stderr(text) => Some(("stderr", text.clone())),
        }
    }
}

/// Desfecho de um ciclo: sucesso com resumo, falha, tempo limite ou cancelamento.
#[derive(Debug, PartialEq, Eq)]
pub enum Exit {
    Success(String),
    Failed(String),
    TimedOut,
    Cancelled,
}

/// Runtime de agente. `on_event` recebe eventos em ordem, enquanto o ciclo roda.
pub trait Runtime {
    fn start(
        &self,
        spec: &AgentSpec<'_>,
        session: &Session<'_>,
        on_event: &mut dyn FnMut(AgentEvent),
    ) -> Exit;

    /// O runtime sabe retomar a sessão de um ciclo anterior ([`AgentSpec::resume`]).
    fn resumes(&self) -> bool {
        false
    }
}
