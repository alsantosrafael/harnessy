//! Adaptador do Codex CLI (`codex exec --json`). Traduz o JSONL da CLI para
//! eventos neutros de [`runtime`](super::runtime); nenhum outro módulo conhece
//! o formato. Leitura roda com `--sandbox read-only`; escrita, com
//! `workspace-write` (só o worktree, sem rede), que é a contenção do Codex.
//!
//! `spec.model` igual a [`guard::CODEX_DEFAULT`] usa o modelo da configuração do próprio Codex;
//! `spec.effort` vira `-c model_reasoning_effort` só para `low`, `medium` e `high`
//! (os níveis `xhigh` e `max` são do Claude e não passam). O Codex
//! não retoma sessão aqui: a correção leva o contexto completo.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    claude::exit_of,
    guard,
    runtime::{AgentEvent, AgentSpec, CycleUsage, Exit, Finish, Runtime, Session},
    supervisor::{self, Line},
};

pub struct Codex {
    binary: PathBuf,
    data_dir: PathBuf,
    env: Vec<(String, String)>,
}

impl Codex {
    /// `env` é o ambiente completo do agente (ver [`guard::env`]).
    pub fn new(binary: PathBuf, data_dir: PathBuf, env: Vec<(String, String)>) -> Self {
        Self {
            binary,
            data_dir,
            env,
        }
    }
}

impl Runtime for Codex {
    fn start(
        &self,
        spec: &AgentSpec<'_>,
        session: &Session<'_>,
        on_event: &mut dyn FnMut(AgentEvent),
    ) -> Exit {
        let schema = match spec
            .json_schema
            .map(|json| write_schema(&self.data_dir, session.run_id, json))
            .transpose()
        {
            Ok(path) => path,
            Err(reason) => return Exit::Failed(reason),
        };
        let plan = supervisor::Plan {
            binary: self.binary.clone(),
            args: guard::codex_args(spec.access, spec.model, spec.effort, schema.as_deref()),
            env: self.env.clone(),
            cwd: spec.dir.to_path_buf(),
            prompt: spec.prompt.to_owned(),
        };
        let mut parser = Parser::default();
        let mut finish: Option<Finish> = None;
        let end = supervisor::run(plan, session.run_id, session.registry, |line| match line {
            Line::Out(text) => {
                for event in parser.parse(&text) {
                    if let AgentEvent::Finished(done) = &event {
                        finish = Some(done.clone());
                    }
                    on_event(event);
                }
            }
            Line::Err(text) => on_event(AgentEvent::Stderr(text)),
        });
        match end {
            Ok(end) => exit_of(&end, finish.as_ref()),
            Err(reason) => Exit::Failed(reason),
        }
    }
}

/// O schema da run fica fora do worktree, onde o agente não edita.
fn write_schema(data_dir: &Path, run_id: i64, schema: &str) -> Result<PathBuf, String> {
    let dir = data_dir.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("run-{run_id}-codex-schema.json"));
    std::fs::write(&path, schema).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Traduz o JSONL do `codex exec --json`; guarda a última mensagem do agente,
/// que com `--output-schema` é a própria saída estruturada.
#[derive(Default)]
struct Parser {
    thread: Option<String>,
    last_message: String,
}

impl Parser {
    fn parse(&mut self, line: &str) -> Vec<AgentEvent> {
        let line = line.trim();
        if line.is_empty() {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![AgentEvent::Log(line.to_owned())];
        };
        let text = |v: &Value, key: &str| v[key].as_str().unwrap_or_default().to_owned();
        match value["type"].as_str() {
            Some("thread.started") => {
                let id = text(&value, "thread_id");
                self.thread = Some(id.clone());
                vec![AgentEvent::Started { session_id: id }]
            }
            Some("item.started") if value["item"]["type"] == "command_execution" => {
                vec![AgentEvent::Tool(text(&value["item"], "command"))]
            }
            Some("item.completed") => match value["item"]["type"].as_str() {
                Some("agent_message") => {
                    self.last_message = text(&value["item"], "text");
                    vec![AgentEvent::Text(self.last_message.clone())]
                }
                Some("error") => vec![AgentEvent::Log(text(&value["item"], "message"))],
                _ => Vec::new(),
            },
            Some("turn.completed") => vec![AgentEvent::Finished(self.finish(&value["usage"]))],
            Some("turn.failed") => vec![AgentEvent::Finished(Finish {
                ok: false,
                subtype: "error".into(),
                summary: text(&value["error"], "message"),
                session_id: self.thread.clone(),
                usage: CycleUsage::default(),
                denials: 0,
                structured: None,
            })],
            Some("error") => vec![AgentEvent::Log(text(&value, "message"))],
            _ => Vec::new(),
        }
    }

    fn finish(&self, usage: &Value) -> Finish {
        let message = self.last_message.trim();
        Finish {
            ok: true,
            subtype: "success".into(),
            summary: self.last_message.clone(),
            session_id: self.thread.clone(),
            usage: CycleUsage {
                input: usage["input_tokens"].as_u64(),
                output: usage["output_tokens"].as_u64(),
                cache_read: usage["cached_input_tokens"].as_u64(),
                cache_write: None,
                cost_usd: None,
                turns: None,
            },
            denials: 0,
            structured: message.starts_with('{').then(|| message.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_all(lines: &[&str]) -> Vec<AgentEvent> {
        let mut parser = Parser::default();
        lines.iter().flat_map(|line| parser.parse(line)).collect()
    }

    #[test]
    fn a_review_turn_becomes_started_text_and_a_finish_with_structured_output() {
        let events = run_all(&[
            r#"{"type":"thread.started","thread_id":"t-1"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.started","item":{"type":"command_execution","command":"git diff"}}"#,
            r#"{"type":"item.completed","item":{"type":"reasoning","text":"..."}}"#,
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"{\"veredito\":\"approve\",\"achados\":[]}"}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":120,"cached_input_tokens":100,"output_tokens":30}}"#,
        ]);
        assert_eq!(
            events[0],
            AgentEvent::Started {
                session_id: "t-1".into()
            }
        );
        assert_eq!(events[1], AgentEvent::Tool("git diff".into()));
        let AgentEvent::Finished(done) = events.last().unwrap() else {
            panic!("sem fim");
        };
        assert!(done.ok);
        assert_eq!(done.session_id.as_deref(), Some("t-1"));
        assert_eq!(
            done.structured.as_deref(),
            Some(r#"{"veredito":"approve","achados":[]}"#)
        );
        assert_eq!(done.usage.input, Some(120));
        assert_eq!(done.usage.cache_read, Some(100));
        assert_eq!(done.usage.output, Some(30));
        assert_eq!(done.usage.cost_usd, None);
    }

    #[test]
    fn prose_is_not_structured_and_a_failed_turn_carries_the_reason() {
        let events = run_all(&[
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"Tudo certo."}}"#,
            r#"{"type":"turn.completed","usage":{}}"#,
        ]);
        let AgentEvent::Finished(done) = events.last().unwrap() else {
            panic!("sem fim");
        };
        assert_eq!(done.structured, None);
        assert_eq!(done.usage, CycleUsage::default());

        let events = run_all(&[
            r#"{"type":"error","message":"reconectando"}"#,
            r#"{"type":"turn.failed","error":{"message":"modelo não suportado"}}"#,
        ]);
        assert_eq!(events[0], AgentEvent::Log("reconectando".into()));
        let AgentEvent::Finished(done) = &events[1] else {
            panic!("sem fim");
        };
        assert!(!done.ok);
        assert_eq!(done.summary, "modelo não suportado");
    }

    #[test]
    fn lines_that_are_not_json_are_logged_and_blank_lines_ignored() {
        let events = run_all(&["", "texto solto", r#"{"type":"desconhecido"}"#]);
        assert_eq!(events, vec![AgentEvent::Log("texto solto".into())]);
    }
}
