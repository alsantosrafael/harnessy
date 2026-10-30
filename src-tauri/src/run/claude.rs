//! Adaptador do Claude Code (`claude -p`): settings, argumentos e ambiente fixos,
//! e tradução do stream-json para eventos neutros de [`runtime`](super::runtime).
//! Nenhum outro módulo conhece o formato desta CLI.

use std::path::{Path, PathBuf};

use super::{
    guard,
    runtime::{Access, AgentEvent, AgentSpec, Exit, Finish, Runtime, Session},
    stream,
    supervisor::{self, End, Line},
};

pub struct ClaudeCode {
    binary: PathBuf,
    data_dir: PathBuf,
    env: Vec<(String, String)>,
}

impl ClaudeCode {
    /// `env` é o ambiente completo do agente (ver [`guard::env`]).
    pub fn new(binary: PathBuf, data_dir: PathBuf, env: Vec<(String, String)>) -> Self {
        Self {
            binary,
            data_dir,
            env,
        }
    }
}

impl Runtime for ClaudeCode {
    fn start(
        &self,
        spec: &AgentSpec<'_>,
        session: &Session<'_>,
        on_event: &mut dyn FnMut(AgentEvent),
    ) -> Exit {
        let settings = match write_settings(&self.data_dir, session.run_id, spec.access) {
            Ok(path) => path,
            Err(reason) => return Exit::Failed(reason),
        };
        let extra = guard::Extra {
            effort: spec.effort,
            resume: spec.resume,
            json_schema: spec.json_schema,
        };
        let plan = supervisor::Plan {
            binary: self.binary.clone(),
            args: guard::args(&settings, spec.model, spec.max_turns, extra),
            env: self.env.clone(),
            cwd: spec.dir.to_path_buf(),
            prompt: spec.prompt.to_owned(),
        };
        let mut finish: Option<Finish> = None;
        let end = supervisor::run(plan, session.run_id, session.registry, |line| match line {
            Line::Out(text) => {
                for event in stream::parse(&text) {
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

    fn resumes(&self) -> bool {
        true
    }
}

/// Desfecho do ciclo a partir de como o processo terminou e do resultado reportado.
pub(super) fn exit_of(end: &End, finish: Option<&Finish>) -> Exit {
    if end.cancelled {
        return Exit::Cancelled;
    }
    if end.timed_out {
        return Exit::TimedOut;
    }
    match finish {
        Some(done) if done.ok && end.status == Some(0) => Exit::Success(done.summary.clone()),
        Some(done) if done.summary.is_empty() => Exit::Failed("O agente terminou com erro.".into()),
        Some(done) => Exit::Failed(done.summary.clone()),
        None => Exit::Failed(format!(
            "O agente terminou sem resultado (código {}).",
            end.status.map_or("?".into(), |c| c.to_string())
        )),
    }
}

/// Settings da run ficam fora do worktree, onde o agente não edita.
fn write_settings(data_dir: &Path, run_id: i64, access: Access) -> Result<PathBuf, String> {
    let dir = data_dir.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("run-{run_id}-settings.json"));
    std::fs::write(&path, guard::settings_json(access)).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Responde se o Claude Code carrega o `CLAUDE.md` do diretório com os
    /// argumentos do Harnessy (`--setting-sources user`). Chama a CLI de verdade
    /// (um turno do `haiku`, sem ferramentas), por isso é ignorado por padrão:
    /// `cargo test instructions_are_loaded_by_the_cli -- --ignored --nocapture`.
    /// Resultado registrado em `docs/development.md` ("Instruções do repositório").
    #[test]
    #[ignore = "chama o claude real e consome tokens"]
    fn instructions_are_loaded_by_the_cli() {
        let dirs = crate::process::search_dirs();
        let binary = crate::process::find_binary("claude", &dirs).expect("claude no PATH");
        let root = std::env::temp_dir().join(format!("harnessy-instr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("CLAUDE.md"),
            "A palavra-senha deste projeto é GIRASSOL-42.\n",
        )
        .unwrap();
        let runtime = ClaudeCode::new(
            binary,
            root.clone(),
            guard::env(&dirs, &root.join("target"), |name| std::env::var(name).ok()),
        );
        let registry = super::super::supervisor::Registry::default();
        registry.register(1);
        let prompt =
            "Responda só com a palavra-senha do projeto, se você a conhece pelo contexto; \
                      senão responda DESCONHECIDA. Não use ferramentas.";
        let spec = AgentSpec::new(&root, prompt, "haiku", 1, Access::Read);
        let session = Session {
            run_id: 1,
            registry: &registry,
        };
        let mut answer = String::new();
        runtime.start(&spec, &session, &mut |event| {
            if let AgentEvent::Finished(done) = event {
                answer = done.summary;
            }
        });
        let _ = std::fs::remove_dir_all(&root);
        println!(
            "CLAUDE.md carregado pela CLI: {}",
            answer.contains("GIRASSOL-42")
        );
    }

    fn end(status: Option<i32>) -> End {
        End {
            status,
            cancelled: false,
            timed_out: false,
        }
    }

    fn finish(ok: bool, summary: &str) -> Finish {
        Finish {
            ok,
            subtype: "success".into(),
            summary: summary.into(),
            session_id: None,
            usage: Default::default(),
            denials: 0,
            structured: None,
        }
    }

    #[test]
    fn success_needs_ok_result_and_zero_status() {
        assert_eq!(
            exit_of(&end(Some(0)), Some(&finish(true, "Pronto."))),
            Exit::Success("Pronto.".into())
        );
        assert_eq!(
            exit_of(&end(Some(1)), Some(&finish(true, "Pronto."))),
            Exit::Failed("Pronto.".into())
        );
        assert_eq!(
            exit_of(&end(Some(0)), Some(&finish(false, ""))),
            Exit::Failed("O agente terminou com erro.".into())
        );
    }

    #[test]
    fn missing_result_reports_the_status_and_cancel_wins() {
        assert_eq!(
            exit_of(&end(Some(3)), None),
            Exit::Failed("O agente terminou sem resultado (código 3).".into())
        );
        let cancelled = End {
            cancelled: true,
            ..end(Some(0))
        };
        assert_eq!(
            exit_of(&cancelled, Some(&finish(true, "ok"))),
            Exit::Cancelled
        );
        let timed = End {
            timed_out: true,
            ..end(None)
        };
        assert_eq!(exit_of(&timed, None), Exit::TimedOut);
    }
}
