//! A decisão de uma etapa, pura: dado o modo, os fatos e as travas, o que o Harnessy faz.
//!
//! Sem banco, sem processo, sem relógio. Quem executa é `flow`. Toda recusa vem com
//! o motivo em uma frase, porque o humano precisa entender por que a automação parou.

use super::{Guardrails, Mode, Stage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    None,
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checks {
    None,
    Pending,
    Passing,
    Failing,
}

/// PR da run, quando existe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub open: bool,
    pub checks: Checks,
    pub conflicting: bool,
}

/// Tudo o que a decisão precisa saber, já reunido por quem chama.
#[derive(Debug, Clone)]
pub struct Facts {
    pub paused: bool,
    pub has_criteria: bool,
    pub other_run_active: bool,
    pub run_completed: bool,
    pub blocked: bool,
    pub proof_failed: bool,
    pub review: Verdict,
    pub review_current: bool,
    /// O QA faz parte do ciclo deste workspace (modo diferente de manual).
    pub qa_expected: bool,
    pub qa: Verdict,
    pub qa_current: bool,
    pub diff_lines: u32,
    pub diff_files: u32,
    pub touched: Vec<String>,
    pub has_worktree: bool,
    pub command_approved: bool,
    pub pr: Option<PullRequest>,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            paused: false,
            has_criteria: true,
            other_run_active: false,
            run_completed: true,
            blocked: false,
            proof_failed: false,
            review: Verdict::None,
            review_current: false,
            qa_expected: false,
            qa: Verdict::None,
            qa_current: false,
            diff_lines: 0,
            diff_files: 0,
            touched: Vec::new(),
            has_worktree: true,
            command_approved: true,
            pr: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Nada a fazer agora: modo manual, ou a etapa já aconteceu.
    Nothing,
    /// Executar sozinho.
    Run,
    /// Preparar e pedir o OK; os avisos são riscos que o humano deve ver antes de aprovar.
    Ask(Vec<String>),
    /// Ainda não dá (falta algo que vai acontecer); volta a decidir quando mudar.
    Wait(String),
    /// Parou e entregou ao humano.
    Hold(String),
}

/// Caminhos alterados que começam por algum prefixo sensível.
pub fn touched_sensitive(touched: &[String], sensitive: &[String]) -> Vec<String> {
    touched
        .iter()
        .filter(|path| {
            sensitive.iter().any(|prefix| {
                path.as_str() == prefix.trim_end_matches('/') || path.starts_with(prefix.as_str())
            })
        })
        .cloned()
        .collect()
}

pub fn decide(stage: Stage, mode: Mode, facts: &Facts, rails: &Guardrails) -> Decision {
    // O merge nunca roda sozinho: o máximo é preparar e pedir o OK.
    let mode = if stage == Stage::Merge && mode == Mode::Auto {
        Mode::Assisted
    } else {
        mode
    };
    if mode == Mode::Manual {
        return Decision::Nothing;
    }
    if mode == Mode::Auto && facts.paused {
        return Decision::Hold("A automação está pausada.".into());
    }
    match hard_stop(stage, facts) {
        Stop::Done => return Decision::Nothing,
        Stop::Wait(reason) => return Decision::Wait(reason),
        Stop::Hold(reason) => return Decision::Hold(reason),
        Stop::Clear => {}
    }
    let warnings = warnings(stage, facts, rails);
    match mode {
        Mode::Auto if warnings.is_empty() => Decision::Run,
        Mode::Auto => Decision::Hold(warnings.join(" ")),
        _ => Decision::Ask(warnings),
    }
}

enum Stop {
    Clear,
    Done,
    Wait(String),
    Hold(String),
}

/// Pré-condições que valem em qualquer modo: sem elas a etapa nem pode ser pedida.
fn hard_stop(stage: Stage, facts: &Facts) -> Stop {
    let wait = |text: &str| Stop::Wait(text.to_owned());
    let hold = |text: &str| Stop::Hold(text.to_owned());
    match stage {
        Stage::Start => {
            if facts.other_run_active {
                return wait("Há outra run ativa; esta tarefa espera a vez.");
            }
            Stop::Clear
        }
        Stage::Review | Stage::Qa | Stage::Publish => {
            if stage == Stage::Publish && facts.pr.is_some() {
                return Stop::Done;
            }
            if !facts.run_completed {
                return wait("A run ainda não terminou.");
            }
            if facts.blocked {
                return hold("A run está bloqueada: resolva o bloqueio antes.");
            }
            Stop::Clear
        }
        Stage::Preview => {
            if !facts.run_completed {
                return wait("A run ainda não terminou.");
            }
            if !facts.has_worktree {
                return hold("O worktree da run não existe mais.");
            }
            Stop::Clear
        }
        Stage::Merge => match &facts.pr {
            None => wait("Publique o PR antes."),
            Some(pr) if !pr.open => Stop::Done,
            Some(pr) if pr.checks == Checks::Failing => hold("Há checks reprovados no PR."),
            Some(pr) if pr.checks == Checks::Pending => {
                wait("Os checks do PR ainda estão rodando.")
            }
            Some(pr) if pr.conflicting => hold("O PR tem conflitos com a base."),
            Some(_) => Stop::Clear,
        },
    }
}

/// Riscos: tiram a etapa do automático e, no assistido, aparecem como aviso na aprovação.
fn warnings(stage: Stage, facts: &Facts, rails: &Guardrails) -> Vec<String> {
    let mut out = Vec::new();
    match stage {
        Stage::Start if !facts.has_criteria => {
            out.push("A tarefa não tem critérios de aceite.".to_owned());
        }
        Stage::Preview if !facts.command_approved => {
            out.push("O comando do projeto ainda não foi confirmado.".to_owned());
        }
        Stage::Publish | Stage::Merge => {
            if facts.diff_lines > rails.max_diff_lines {
                out.push(format!(
                    "O diff tem {} linhas (limite {}).",
                    facts.diff_lines, rails.max_diff_lines
                ));
            }
            if facts.diff_files > rails.max_files {
                out.push(format!(
                    "O diff toca {} arquivos (limite {}).",
                    facts.diff_files, rails.max_files
                ));
            }
            let touched = touched_sensitive(&facts.touched, &rails.sensitive);
            if !touched.is_empty() {
                out.push(format!("Toca caminhos sensíveis: {}.", touched.join(", ")));
            }
            if facts.review == Verdict::RequestChanges {
                out.push("A revisão pediu mudanças.".to_owned());
            }
            if facts.qa == Verdict::RequestChanges {
                out.push("O QA encontrou problemas.".to_owned());
            }
            if stage == Stage::Publish && facts.proof_failed {
                out.push("A contraprova não provou os testes novos.".to_owned());
            }
            if stage == Stage::Merge {
                if facts.review != Verdict::Approve || !facts.review_current {
                    out.push("Falta a aprovação do staff para o commit atual.".to_owned());
                }
                if facts.qa_expected && (facts.qa != Verdict::Approve || !facts.qa_current) {
                    out.push("Falta a aprovação do QA para o commit atual.".to_owned());
                }
            }
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rails() -> Guardrails {
        Guardrails::default()
    }

    fn open_pr(checks: Checks) -> Option<PullRequest> {
        Some(PullRequest {
            open: true,
            checks,
            conflicting: false,
        })
    }

    #[test]
    fn manual_never_acts_and_the_merge_is_never_automatic() {
        let facts = Facts {
            pr: open_pr(Checks::Passing),
            review: Verdict::Approve,
            review_current: true,
            ..Facts::default()
        };
        assert_eq!(
            decide(Stage::Publish, Mode::Manual, &facts, &rails()),
            Decision::Nothing
        );
        assert_eq!(
            decide(Stage::Merge, Mode::Auto, &facts, &rails()),
            Decision::Ask(vec![])
        );
    }

    #[test]
    fn auto_runs_only_when_nothing_is_risky_and_otherwise_hands_back_with_the_reason() {
        let clean = Facts::default();
        assert_eq!(
            decide(Stage::Publish, Mode::Auto, &clean, &rails()),
            Decision::Run
        );
        let big = Facts {
            diff_lines: 900,
            diff_files: 30,
            ..Facts::default()
        };
        let Decision::Hold(reason) = decide(Stage::Publish, Mode::Auto, &big, &rails()) else {
            panic!("devia parar");
        };
        assert!(
            reason.contains("900 linhas") && reason.contains("30 arquivos"),
            "{reason}"
        );
        let sensitive = Facts {
            touched: vec!["src-tauri/migrations/001.sql".into(), "src/a.ts".into()],
            ..Facts::default()
        };
        let Decision::Hold(reason) = decide(Stage::Publish, Mode::Auto, &sensitive, &rails())
        else {
            panic!("devia parar");
        };
        assert!(reason.contains("src-tauri/migrations/001.sql") && !reason.contains("src/a.ts"));
    }

    #[test]
    fn assisted_asks_and_shows_the_same_risks_as_warnings() {
        let risky = Facts {
            proof_failed: true,
            review: Verdict::RequestChanges,
            ..Facts::default()
        };
        let Decision::Ask(warnings) = decide(Stage::Publish, Mode::Assisted, &risky, &rails())
        else {
            panic!("devia pedir OK");
        };
        assert_eq!(warnings.len(), 2);
        assert_eq!(
            decide(Stage::Publish, Mode::Assisted, &Facts::default(), &rails()),
            Decision::Ask(vec![])
        );
    }

    #[test]
    fn pause_stops_delegated_steps_but_not_the_requests_for_approval() {
        let paused = Facts {
            paused: true,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Preview, Mode::Auto, &paused, &rails()),
            Decision::Hold(_)
        ));
        assert!(matches!(
            decide(Stage::Preview, Mode::Assisted, &paused, &rails()),
            Decision::Ask(_)
        ));
    }

    #[test]
    fn steps_wait_for_the_run_and_hold_when_it_is_blocked() {
        let running = Facts {
            run_completed: false,
            ..Facts::default()
        };
        for stage in [Stage::Review, Stage::Qa, Stage::Preview, Stage::Publish] {
            assert!(
                matches!(
                    decide(stage, Mode::Auto, &running, &rails()),
                    Decision::Wait(_)
                ),
                "{stage:?}"
            );
        }
        let blocked = Facts {
            blocked: true,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Publish, Mode::Auto, &blocked, &rails()),
            Decision::Hold(_)
        ));
        assert_eq!(
            decide(Stage::Preview, Mode::Auto, &blocked, &rails()),
            Decision::Run,
            "ver a mudança não exige run sem bloqueio"
        );
    }

    #[test]
    fn start_waits_its_turn_and_wants_acceptance_criteria() {
        let busy = Facts {
            other_run_active: true,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Start, Mode::Auto, &busy, &rails()),
            Decision::Wait(_)
        ));
        let vague = Facts {
            has_criteria: false,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Start, Mode::Auto, &vague, &rails()),
            Decision::Hold(_)
        ));
        assert_eq!(
            decide(Stage::Start, Mode::Auto, &Facts::default(), &rails()),
            Decision::Run
        );
    }

    #[test]
    fn preview_in_auto_needs_the_command_to_be_confirmed() {
        let unconfirmed = Facts {
            command_approved: false,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Preview, Mode::Auto, &unconfirmed, &rails()),
            Decision::Hold(_)
        ));
        assert!(
            matches!(decide(Stage::Preview, Mode::Assisted, &unconfirmed, &rails()), Decision::Ask(w) if w.len() == 1)
        );
        let gone = Facts {
            has_worktree: false,
            ..Facts::default()
        };
        assert!(matches!(
            decide(Stage::Preview, Mode::Assisted, &gone, &rails()),
            Decision::Hold(_)
        ));
    }

    #[test]
    fn publishing_twice_is_nothing() {
        let published = Facts {
            pr: open_pr(Checks::None),
            ..Facts::default()
        };
        assert_eq!(
            decide(Stage::Publish, Mode::Auto, &published, &rails()),
            Decision::Nothing
        );
    }

    #[test]
    fn merge_waits_for_checks_stops_on_failures_and_asks_with_the_missing_approvals() {
        let merge = |facts: &Facts| decide(Stage::Merge, Mode::Assisted, facts, &rails());
        assert!(
            matches!(merge(&Facts::default()), Decision::Wait(_)),
            "sem PR"
        );
        assert!(matches!(
            merge(&Facts {
                pr: open_pr(Checks::Pending),
                ..Facts::default()
            }),
            Decision::Wait(_)
        ));
        assert!(matches!(
            merge(&Facts {
                pr: open_pr(Checks::Failing),
                ..Facts::default()
            }),
            Decision::Hold(_)
        ));
        let conflict = Some(PullRequest {
            open: true,
            checks: Checks::Passing,
            conflicting: true,
        });
        assert!(matches!(
            merge(&Facts {
                pr: conflict,
                ..Facts::default()
            }),
            Decision::Hold(_)
        ));
        let merged = Some(PullRequest {
            open: false,
            checks: Checks::Passing,
            conflicting: false,
        });
        assert_eq!(
            merge(&Facts {
                pr: merged,
                ..Facts::default()
            }),
            Decision::Nothing
        );

        let unreviewed = Facts {
            pr: open_pr(Checks::Passing),
            qa_expected: true,
            ..Facts::default()
        };
        let Decision::Ask(warnings) = merge(&unreviewed) else {
            panic!("devia pedir OK")
        };
        assert_eq!(warnings.len(), 2, "staff e QA");
        let ready = Facts {
            pr: open_pr(Checks::Passing),
            review: Verdict::Approve,
            review_current: true,
            qa_expected: true,
            qa: Verdict::Approve,
            qa_current: true,
            ..Facts::default()
        };
        assert_eq!(merge(&ready), Decision::Ask(vec![]));
        let stale = Facts {
            review_current: false,
            ..ready
        };
        assert!(
            matches!(merge(&stale), Decision::Ask(w) if w.len() == 1),
            "commit novo derruba a aprovação"
        );
    }

    #[test]
    fn sensitive_prefixes_match_paths_and_exact_files() {
        let sensitive = vec!["package.json".to_owned(), ".github/".to_owned()];
        let touched = vec![
            "package.json".into(),
            ".github/workflows/ci.yml".into(),
            "src/package.json.ts".into(),
            "docs/a.md".into(),
        ];
        assert_eq!(
            touched_sensitive(&touched, &sensitive),
            [
                "package.json",
                ".github/workflows/ci.yml",
                "src/package.json.ts"
            ]
            .iter()
            .take(2)
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
        );
    }
}
