//! Ação de leitura: o agente responde sobre o repositório com acesso só de leitura.
//! Não há worktree de escrita, checks nem ciclo de correção. A resposta vira o
//! resumo da run e é opinião do agente, nunca evidência; por isso este caminho
//! não move o card (ver `AppSink` em `commands.rs`).

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{Job, Sink};
use crate::{
    db,
    run::{
        self,
        runtime::{Access, AgentEvent, AgentSpec, Exit, Finish, Runtime, Session},
        supervisor::Registry,
        CycleRecord,
    },
};

/// Resposta longa demais é cortada; o texto completo fica no log da run.
const MAX_ANSWER_BYTES: usize = 16_000;
const NO_ANSWER: &str = "O agente não respondeu.";
const MAX_STEPS: usize = 5;
const MAX_QUESTIONS: usize = 6;
const MAX_CRITERIA: usize = 12;
const MAX_REFINED_BYTES: usize = 8_000;

/// Formato pedido a "Dividir em passos" (`saida = "passos"`).
pub const STEPS_SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"required":["passos"],"properties":{"passos":{"type":"array","minItems":1,"maxItems":5,"items":{"type":"object","additionalProperties":false,"required":["titulo","criterios"],"properties":{"titulo":{"type":"string"},"criterios":{"type":"array","minItems":1,"maxItems":4,"items":{"type":"string"}}}}}}}"#;

/// Formato pedido a "Refinar" (`saida = "refinamento"`).
pub const REFINE_SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"required":["pronta","perguntas","descricao","criterios"],"properties":{"pronta":{"type":"boolean"},"perguntas":{"type":"array","maxItems":6,"items":{"type":"object","additionalProperties":false,"required":["pergunta","recomendada"],"properties":{"pergunta":{"type":"string"},"recomendada":{"type":"string"}}}},"descricao":{"type":"string"},"criterios":{"type":"array","maxItems":12,"items":{"type":"string"}}}}"#;

#[derive(Deserialize)]
struct RawRefinement {
    #[serde(default)]
    pronta: bool,
    #[serde(default)]
    perguntas: Vec<RawQuestion>,
    #[serde(default)]
    descricao: String,
    #[serde(default)]
    criterios: Vec<String>,
}

#[derive(Deserialize)]
struct RawQuestion {
    pergunta: String,
    #[serde(default)]
    recomendada: String,
}

/// Pergunta em aberto, com a resposta que o agente recomenda.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Question {
    question: String,
    recommended: String,
}

/// Rascunho de refinamento, no formato que a interface edita. Os critérios já
/// vêm como linhas `- item`, a convenção dos critérios da tarefa.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Refinement {
    /// Nenhuma pergunta em aberto mudaria o rascunho.
    ready: bool,
    questions: Vec<Question>,
    description: String,
    criteria: String,
}

/// Refinamento da saída estruturada (ou do primeiro objeto JSON do texto).
/// `None` quando nada aproveitável veio.
pub(super) fn refinement(structured: Option<&str>, text: &str) -> Option<Refinement> {
    let raw = structured
        .and_then(|json| serde_json::from_str::<RawRefinement>(json).ok())
        .or_else(|| {
            let start = text.find('{')?;
            let end = text.rfind('}')?;
            serde_json::from_str::<RawRefinement>(text.get(start..=end)?).ok()
        })?;
    let questions: Vec<Question> = raw
        .perguntas
        .into_iter()
        .filter(|q| !q.pergunta.trim().is_empty())
        .take(MAX_QUESTIONS)
        .map(|q| Question {
            question: q.pergunta.trim().to_owned(),
            recommended: q.recomendada.trim().to_owned(),
        })
        .collect();
    let description = clip_to(raw.descricao.trim(), MAX_REFINED_BYTES);
    let criteria = clip_to(
        &raw.criterios
            .iter()
            .filter(|c| !c.trim().is_empty())
            .take(MAX_CRITERIA)
            .map(|c| format!("- {}", c.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        MAX_REFINED_BYTES,
    );
    // "Pronta" com perguntas pendentes é contraditório: vale o que falta decidir.
    let ready = raw.pronta && questions.is_empty();
    (!questions.is_empty() || !description.is_empty() || !criteria.is_empty()).then_some(
        Refinement {
            ready,
            questions,
            description,
            criteria,
        },
    )
}

#[derive(Deserialize)]
struct RawSteps {
    passos: Vec<RawStep>,
}

#[derive(Deserialize)]
struct RawStep {
    titulo: String,
    criterios: Vec<String>,
}

/// Passo proposto, no formato que a interface edita e devolve a `create_steps`.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Proposal {
    title: String,
    criteria: String,
}

/// Passos da saída estruturada (ou do primeiro objeto JSON do texto). `None`
/// quando nada válido veio; passos sem título são descartados.
pub(super) fn proposals(structured: Option<&str>, text: &str) -> Option<Vec<Proposal>> {
    let raw = structured
        .and_then(|json| serde_json::from_str::<RawSteps>(json).ok())
        .or_else(|| {
            let start = text.find('{')?;
            let end = text.rfind('}')?;
            serde_json::from_str::<RawSteps>(text.get(start..=end)?).ok()
        })?;
    let steps: Vec<Proposal> = raw
        .passos
        .into_iter()
        .filter(|step| !step.titulo.trim().is_empty())
        .take(MAX_STEPS)
        .map(|step| Proposal {
            title: step.titulo.trim().to_owned(),
            criteria: step
                .criterios
                .iter()
                .filter(|c| !c.trim().is_empty())
                .map(|c| format!("- {}", c.trim()))
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect();
    (!steps.is_empty()).then_some(steps)
}

/// Roda a Ação de leitura até o fim. `job.repo` é a pasta do workspace; checks e
/// correção não se aplicam.
pub fn execute(job: Job, registry: &Registry, sink: &dyn Sink, runtime: &dyn Runtime) {
    let id = job.run_id;
    sink.state(id, run::IMPLEMENTING, None);
    sink.event(id, "status", "Agente em execução (só leitura)…");

    let mut spec = AgentSpec::new(
        &job.repo,
        &job.prompt,
        &job.model,
        job.max_turns,
        Access::Read,
    );
    spec.effort = job.effort.as_deref();
    spec.json_schema = job.output_schema;
    let session = Session {
        run_id: id,
        registry,
    };
    let started_at = db::now();
    let started = Instant::now();
    let mut finish_event: Option<Finish> = None;
    let mut session_id: Option<String> = None;
    let exit = runtime.start(&spec, &session, &mut |event| {
        match &event {
            AgentEvent::Started { session_id: sid } => session_id = Some(sid.clone()),
            AgentEvent::Finished(done) => finish_event = Some(done.clone()),
            _ => {}
        }
        if let Some((kind, text)) = event.log_line() {
            sink.event(id, kind, &text);
        }
    });

    let usage = finish_event
        .as_ref()
        .map(|done| done.usage)
        .unwrap_or_default();
    let session_id = finish_event
        .as_ref()
        .and_then(|done| done.session_id.clone())
        .or(session_id);
    sink.cycle(
        id,
        &CycleRecord {
            cycle: 1,
            model: &job.model,
            session_id: session_id.as_deref(),
            resumed: false,
            usage,
            trigger: run::TRIGGER_INITIAL,
            started_at,
            finished_at: db::now(),
            agent_ms: started.elapsed().as_millis() as i64,
        },
    );
    registry.unregister(id);

    if job.output_schema == Some(REFINE_SCHEMA) {
        if let Exit::Success(text) = &exit {
            let structured = finish_event
                .as_ref()
                .and_then(|done| done.structured.as_deref());
            let summary = match refinement(structured, text) {
                Some(draft) => {
                    if let Ok(json) = serde_json::to_string(&draft) {
                        sink.event(id, "refinement", &json);
                    }
                    if draft.ready {
                        "Tarefa pronta para implementar. Revise o rascunho e aplique à tarefa."
                            .to_owned()
                    } else {
                        format!(
                            "{} pergunta(s) em aberto. Revise o rascunho, responda e refine de novo ou aplique à tarefa.",
                            draft.questions.len()
                        )
                    }
                }
                None => "O agente não devolveu o formato esperado.".to_owned(),
            };
            sink.state(id, run::COMPLETED, Some(&summary));
            return;
        }
    }
    if job.output_schema.is_some() {
        if let Exit::Success(text) = &exit {
            let structured = finish_event
                .as_ref()
                .and_then(|done| done.structured.as_deref());
            let summary = match proposals(structured, text) {
                Some(steps) => {
                    if let Ok(json) = serde_json::to_string(&steps) {
                        sink.event(id, "steps", &json);
                    }
                    let list: Vec<String> = steps
                        .iter()
                        .enumerate()
                        .map(|(at, step)| format!("{}. {}", at + 1, step.title))
                        .collect();
                    format!(
                        "{} passos propostos. Revise e confirme para criar as tarefas.\n{}",
                        steps.len(),
                        list.join("\n")
                    )
                }
                None => "O agente não devolveu o formato esperado.".to_owned(),
            };
            sink.state(id, run::COMPLETED, Some(&summary));
            return;
        }
    }
    match exit {
        Exit::Success(text) if text.trim().is_empty() => {
            sink.state(id, run::COMPLETED, Some(NO_ANSWER));
        }
        Exit::Success(text) => sink.state(id, run::COMPLETED, Some(&clip(&text))),
        Exit::Failed(reason) => sink.state(id, run::FAILED, Some(&clip(&reason))),
        Exit::TimedOut => sink.state(id, run::FAILED, Some("Tempo limite da run excedido.")),
        Exit::Cancelled => sink.state(id, run::CANCELLED, Some("Run cancelada.")),
    }
}

fn clip(text: &str) -> String {
    clip_to(text, MAX_ANSWER_BYTES)
}

fn clip_to(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// Agente falso que só responde; guarda o acesso que recebeu.
    struct ReadAgent {
        access: Cell<Option<Access>>,
        answer: &'static str,
    }

    impl Runtime for ReadAgent {
        fn start(
            &self,
            spec: &AgentSpec<'_>,
            _session: &Session<'_>,
            _on_event: &mut dyn FnMut(AgentEvent),
        ) -> Exit {
            self.access.set(Some(spec.access));
            Exit::Success(self.answer.into())
        }
    }

    /// Sink que registra estados e ciclos, sem banco.
    #[derive(Default)]
    struct Recorder {
        states: RefCell<Vec<(String, Option<String>)>>,
        cycles: Cell<u32>,
    }

    impl Sink for Recorder {
        fn event(&self, _: i64, _: &str, _: &str) {}
        fn state(&self, _: i64, state: &str, summary: Option<&str>) {
            self.states
                .borrow_mut()
                .push((state.into(), summary.map(str::to_owned)));
        }
        fn blocked(&self, _: i64, _: &str) {
            panic!("leitura não bloqueia a run");
        }
        fn worktree(&self, _: i64, _: &str, _: &str, _: &str) {
            panic!("leitura não cria worktree");
        }
        fn cycle(&self, _: i64, _: &CycleRecord<'_>) {
            self.cycles.set(self.cycles.get() + 1);
        }
        fn checks_ms(&self, _: i64, _: i64) {
            panic!("leitura não roda checks");
        }
        fn proof(&self, _: i64, _: &str) {
            panic!("leitura não roda contraprova");
        }
    }

    fn job(repo: &std::path::Path) -> Job {
        Job {
            run_id: 7,
            title: "Explicar".into(),
            prompt: "Explique.".into(),
            repo: repo.to_path_buf(),
            data_dir: repo.to_path_buf(),
            model: "haiku".into(),
            checks: Vec::new(),
            max_corrections: 0,
            contraprova: false,
            require_proof: false,
            effort: None,
            max_turns: crate::run::guard::MAX_TURNS,
            review: None,
            base_ref: None,
            output_schema: None,
        }
    }

    #[test]
    fn read_run_answers_in_place_with_read_access_and_no_checks() {
        let agent = ReadAgent {
            access: Cell::new(None),
            answer: "Troque a função X no arquivo Y.",
        };
        let sink = Recorder::default();
        let registry = Registry::default();
        registry.register(7);
        execute(job(std::path::Path::new("/tmp")), &registry, &sink, &agent);

        assert_eq!(agent.access.get(), Some(Access::Read));
        assert_eq!(sink.cycles.get(), 1);
        let states = sink.states.borrow();
        let (last, summary) = states.last().unwrap();
        assert_eq!(last, run::COMPLETED);
        assert_eq!(summary.as_deref(), Some("Troque a função X no arquivo Y."));
    }

    #[test]
    fn step_proposals_come_from_structured_output_and_are_capped() {
        let json = r#"{"passos":[{"titulo":" Criar tipo ","criterios":["compila","tem teste"]},{"titulo":"","criterios":["x"]},{"titulo":"Usar tipo","criterios":["tela mostra"]}]}"#;
        let steps = proposals(Some(json), "").unwrap();
        assert_eq!(
            steps,
            vec![
                Proposal {
                    title: "Criar tipo".into(),
                    criteria: "- compila\n- tem teste".into()
                },
                Proposal {
                    title: "Usar tipo".into(),
                    criteria: "- tela mostra".into()
                },
            ]
        );
        assert!(proposals(None, "nada").is_none());
        assert!(proposals(Some(r#"{"passos":[]}"#), "").is_none());
        let many: Vec<String> = (0..9)
            .map(|i| format!(r#"{{"titulo":"p{i}","criterios":["c"]}}"#))
            .collect();
        let text = format!(r#"Aqui: {{"passos":[{}]}}"#, many.join(","));
        assert_eq!(proposals(None, &text).unwrap().len(), MAX_STEPS);
        let _: serde_json::Value = serde_json::from_str(STEPS_SCHEMA).unwrap();
    }

    #[test]
    fn refinement_comes_from_structured_output_and_is_capped() {
        let json = r#"{"perguntas":[{"pergunta":" Quem usa? ","recomendada":" Qualquer pessoa "},{"pergunta":"  ","recomendada":"x"}],"descricao":" Objetivo claro ","criterios":["abre a tela","","salva"]}"#;
        let draft = refinement(Some(json), "").unwrap();
        assert_eq!(
            draft,
            Refinement {
                ready: false,
                questions: vec![Question {
                    question: "Quem usa?".into(),
                    recommended: "Qualquer pessoa".into()
                }],
                description: "Objetivo claro".into(),
                criteria: "- abre a tela\n- salva".into(),
            }
        );
        assert!(refinement(None, "nada").is_none());
        // "Pronta" com pergunta pendente não vale como pronta.
        let both = r#"{"pronta":true,"perguntas":[{"pergunta":"Q?","recomendada":"Sim"}],"descricao":"D","criterios":[]}"#;
        assert!(!refinement(Some(both), "").unwrap().ready);
        let ready = r#"{"pronta":true,"perguntas":[],"descricao":"D","criterios":["c"]}"#;
        assert!(refinement(Some(ready), "").unwrap().ready);
        assert!(refinement(
            Some(r#"{"perguntas":[],"descricao":"","criterios":[]}"#),
            ""
        )
        .is_none());
        let many: Vec<String> = (0..9)
            .map(|i| format!(r#"{{"pergunta":"p{i}","recomendada":"r"}}"#))
            .collect();
        let text = format!(r#"Aqui: {{"perguntas":[{}]}}"#, many.join(","));
        assert_eq!(
            refinement(None, &text).unwrap().questions.len(),
            MAX_QUESTIONS
        );
        let _: serde_json::Value = serde_json::from_str(REFINE_SCHEMA).unwrap();
    }

    #[test]
    fn refinement_run_emits_the_draft_and_never_blocks_or_moves_anything() {
        let agent = ReadAgent {
            access: Cell::new(None),
            answer: r#"{"pronta":false,"perguntas":[{"pergunta":"Para quem?","recomendada":"Todos"}],"descricao":"D","criterios":["c"]}"#,
        };
        let sink = Recorder::default();
        let registry = Registry::default();
        registry.register(7);
        let mut refine = job(std::path::Path::new("/tmp"));
        refine.output_schema = Some(REFINE_SCHEMA);
        execute(refine, &registry, &sink, &agent);

        assert_eq!(agent.access.get(), Some(Access::Read));
        let states = sink.states.borrow();
        let (last, summary) = states.last().unwrap();
        assert_eq!(last, run::COMPLETED);
        assert!(summary.as_deref().unwrap().starts_with("1 pergunta(s)"));
    }

    #[test]
    fn empty_answer_gets_a_visible_note() {
        let agent = ReadAgent {
            access: Cell::new(None),
            answer: "  ",
        };
        let sink = Recorder::default();
        let registry = Registry::default();
        registry.register(7);
        execute(job(std::path::Path::new("/tmp")), &registry, &sink, &agent);
        let states = sink.states.borrow();
        assert_eq!(states.last().unwrap().1.as_deref(), Some(NO_ANSWER));
    }
}
