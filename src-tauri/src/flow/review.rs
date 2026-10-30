//! Revisor: Ação de leitura sobre o resultado de uma run verde (SPEC §6.4).
//!
//! Recebe só tarefa, critérios, checks, contraprova e diff — nunca a sessão do
//! implementador — e devolve veredito e achados por `json_schema`. O núcleo
//! descarta todo achado que não aponta para uma linha alterada do diff (filtro
//! determinístico contra alucinação do revisor). Achados "bloqueia" que sobram
//! viram `Failure` e entram no ciclo de correção normal.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{failure::Failure, proof::Verdict};
use crate::{
    action::{Action, Contexto, Quando},
    run::{
        guard, prompt,
        runtime::{Access, Runtime},
        worktree,
    },
};

/// Linhas de contexto aceitas ao redor de um trecho alterado.
const CONTEXT_LINES: u32 = 3;
/// Diff acima disto (linhas adicionadas + removidas) é sinal de risco.
const RISKY_DIFF_LINES: usize = 300;
const MAX_FINDINGS: usize = 10;
pub(super) const CHECK_NAME: &str = "revisão";

/// Formato pedido ao runtime (`--json-schema`).
pub(super) const SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"required":["veredito","achados"],"properties":{"veredito":{"type":"string","enum":["approve","request_changes"]},"achados":{"type":"array","maxItems":10,"items":{"type":"object","additionalProperties":false,"required":["arquivo","linha","severidade","texto"],"properties":{"arquivo":{"type":"string"},"linha":{"type":"integer","minimum":1},"severidade":{"type":"string","enum":["bloqueia","sugere"]},"texto":{"type":"string"}}}}}}"#;

/// Revisor configurado para uma run de escrita: a Ação `revisar` (ou a do
/// usuário com o mesmo id) e o modelo já decidido.
pub struct Reviewer {
    pub(crate) action: Action,
    pub model: String,
    pub tarefa: String,
    pub descricao: String,
    pub criterios: String,
    /// Runtime próprio do revisor (o Codex); `None` usa o do implementador.
    pub runtime: Option<Box<dyn Runtime + Send>>,
}

impl Reviewer {
    /// Modelo do revisor: o da Ação, senão um degrau acima do implementador.
    pub fn model_for(action: &Action, implementer: &str) -> String {
        action
            .modelo
            .clone()
            .or_else(|| guard::next_model(implementer).map(str::to_owned))
            .unwrap_or_else(|| implementer.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Finding {
    pub(super) arquivo: String,
    pub(super) linha: u32,
    pub(super) severidade: String,
    pub(super) texto: String,
}

#[derive(Debug, Deserialize)]
struct Raw {
    veredito: String,
    #[serde(default)]
    achados: Vec<Finding>,
}

/// Resultado da revisão depois do filtro.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Outcome {
    /// `approve`, `request_changes` ou `invalid` (saída fora do formato).
    pub(super) verdict: &'static str,
    pub(super) blocking: Vec<Finding>,
    pub(super) suggestions: Vec<Finding>,
    pub(super) discarded: usize,
}

impl Outcome {
    pub(super) fn failures(&self) -> Vec<Failure> {
        self.blocking
            .iter()
            .map(|finding| Failure {
                check: CHECK_NAME.into(),
                file: Some(finding.arquivo.clone()),
                line: Some(finding.linha),
                test_name: None,
                expected: None,
                actual: None,
                excerpt: finding.texto.clone(),
            })
            .collect()
    }

    pub(super) fn message(&self) -> String {
        let discarded = if self.discarded > 0 {
            format!(" ({} achado(s) fora do diff descartado(s))", self.discarded)
        } else {
            String::new()
        };
        match self.verdict {
            "request_changes" => format!(
                "Revisão pediu mudanças: {} ponto(s) que bloqueiam{discarded}",
                self.blocking.len()
            ),
            "approve" if self.suggestions.is_empty() => format!("Revisão aprovou{discarded}"),
            "approve" => format!(
                "Revisão aprovou com {} sugestão(ões){discarded}",
                self.suggestions.len()
            ),
            _ => "O revisor não devolveu o formato esperado; resultado sem revisão".into(),
        }
    }

    pub(super) fn event_json(&self) -> String {
        json!({
            "verdict": self.verdict,
            "blocking": self.blocking,
            "suggestions": self.suggestions,
            "discarded": self.discarded,
        })
        .to_string()
    }
}

/// O revisor roda? Só com run verde (quem chama garante) e conforme `quando`.
pub(super) fn is_due(quando: Quando, proof: Verdict, diff_lines: usize) -> bool {
    match quando {
        Quando::Manual => false,
        Quando::Sempre => true,
        Quando::Risco => {
            matches!(
                proof,
                Verdict::NotProved | Verdict::NoTests | Verdict::Inconclusive
            ) || diff_lines > RISKY_DIFF_LINES
        }
    }
}

/// Linhas adicionadas e removidas num diff unificado.
pub(super) fn diff_size(diff: &str) -> usize {
    diff.lines()
        .filter(|line| {
            (line.starts_with('+') && !line.starts_with("+++"))
                || (line.starts_with('-') && !line.starts_with("---"))
        })
        .count()
}

/// Trechos alterados por arquivo, em linhas do arquivo novo, a partir de um
/// diff `-U0`. Arquivo apagado não entra (não há linha nova para apontar).
pub(super) fn changed_lines(diff_u0: &str) -> HashMap<String, Vec<(u32, u32)>> {
    let mut map: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in diff_u0.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            current = path.strip_prefix("b/").map(str::to_owned);
        } else if let Some(hunk) = line.strip_prefix("@@ ") {
            let (Some(file), Some(new)) = (
                current.as_ref(),
                hunk.split_whitespace().find(|part| part.starts_with('+')),
            ) else {
                continue;
            };
            let mut parts = new[1..].splitn(2, ',');
            let start: u32 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
            let count: u32 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(1);
            // Remoção pura (`+n,0`) aponta para a linha onde o trecho sumiu.
            let end = if count == 0 { start } else { start + count - 1 };
            map.entry(file.clone()).or_default().push((start, end));
        }
    }
    map
}

/// Interpreta a saída do revisor (estruturada ou, em último caso, o primeiro
/// objeto JSON do texto), filtra achados fora do diff e separa por severidade.
pub(super) fn evaluate(
    structured: Option<&str>,
    text: &str,
    changed: &HashMap<String, Vec<(u32, u32)>>,
) -> Outcome {
    let raw = structured
        .and_then(|json| serde_json::from_str::<Raw>(json).ok())
        .or_else(|| {
            let start = text.find('{')?;
            let end = text.rfind('}')?;
            serde_json::from_str::<Raw>(text.get(start..=end)?).ok()
        });
    let Some(raw) =
        raw.filter(|raw| raw.veredito == "approve" || raw.veredito == "request_changes")
    else {
        return Outcome {
            verdict: "invalid",
            blocking: Vec::new(),
            suggestions: Vec::new(),
            discarded: 0,
        };
    };
    let mut blocking = Vec::new();
    let mut suggestions = Vec::new();
    let mut discarded = 0;
    for finding in raw.achados.into_iter().take(MAX_FINDINGS) {
        let path = finding.arquivo.trim_start_matches("./").to_owned();
        let inside = changed.get(&path).is_some_and(|ranges| {
            ranges.iter().any(|(start, end)| {
                finding.linha + CONTEXT_LINES >= *start && finding.linha <= end + CONTEXT_LINES
            })
        });
        if !inside {
            discarded += 1;
        } else if finding.severidade == "bloqueia" {
            blocking.push(Finding {
                arquivo: path,
                ..finding
            });
        } else {
            suggestions.push(Finding {
                arquivo: path,
                ..finding
            });
        }
    }
    let verdict = if raw.veredito == "request_changes" && !blocking.is_empty() {
        "request_changes"
    } else {
        "approve"
    };
    Outcome {
        verdict,
        blocking,
        suggestions,
        discarded,
    }
}

/// Diffs que a revisão precisa: o do prompt (com contexto) e o `-U0` do filtro.
pub(super) struct Diffs {
    pub(super) unified: String,
    pub(super) changed: HashMap<String, Vec<(u32, u32)>>,
    pub(super) size: usize,
}

pub(super) fn diffs(git: &Path, dirs: &[PathBuf], dir: &Path, base: &str) -> Result<Diffs, String> {
    let unified = worktree::diff_text(git, dirs, dir, base, CONTEXT_LINES)?;
    let zero = worktree::diff_text(git, dirs, dir, base, 0)?;
    Ok(Diffs {
        size: diff_size(&zero),
        changed: changed_lines(&zero),
        unified,
    })
}

/// Prompt completo do revisor: início fixo de leitura e o pedido da Ação.
pub(super) fn prompt_for(reviewer: &Reviewer, diff: &str, checks: &str, proof: &str) -> String {
    let ctx = Contexto {
        tarefa: &reviewer.tarefa,
        descricao: &reviewer.descricao,
        criterios: &reviewer.criterios,
        diff,
        checks,
        contraprova: proof,
        ..Default::default()
    };
    prompt::compose(Access::Read, &reviewer.action.render(&ctx), &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF_U0: &str = "diff --git a/src/a.ts b/src/a.ts
index 1..2 100644
--- a/src/a.ts
+++ b/src/a.ts
@@ -10,0 +11,3 @@ fn x
+um
+dois
+três
@@ -40 +43 @@
-velho
+novo
diff --git a/old.txt b/old.txt
deleted file mode 100644
--- a/old.txt
+++ /dev/null
@@ -1 +0,0 @@
-tchau
";

    fn finding(arquivo: &str, linha: u32, severidade: &str) -> serde_json::Value {
        json!({"arquivo": arquivo, "linha": linha, "severidade": severidade, "texto": "x"})
    }

    #[test]
    fn changed_lines_follow_the_new_file_and_skip_deleted_files() {
        let changed = changed_lines(DIFF_U0);
        assert_eq!(changed["src/a.ts"], vec![(11, 13), (43, 43)]);
        assert!(!changed.contains_key("old.txt"));
        assert_eq!(diff_size(DIFF_U0), 6);
    }

    #[test]
    fn findings_outside_the_diff_are_discarded_and_only_blocking_ones_request_changes() {
        let changed = changed_lines(DIFF_U0);
        let output = json!({
            "veredito": "request_changes",
            "achados": [
                finding("src/a.ts", 12, "bloqueia"),
                finding("./src/a.ts", 45, "sugere"),
                finding("src/a.ts", 200, "bloqueia"),
                finding("src/inventado.ts", 1, "bloqueia"),
            ]
        })
        .to_string();
        let outcome = evaluate(Some(&output), "", &changed);
        assert_eq!(outcome.verdict, "request_changes");
        assert_eq!(outcome.blocking.len(), 1);
        assert_eq!(outcome.suggestions[0].arquivo, "src/a.ts");
        assert_eq!(outcome.discarded, 2);
        let failures = outcome.failures();
        assert_eq!(failures[0].check, CHECK_NAME);
        assert_eq!(failures[0].line, Some(12));
    }

    #[test]
    fn request_changes_without_surviving_blockers_becomes_approve() {
        let changed = changed_lines(DIFF_U0);
        let output =
            json!({"veredito": "request_changes", "achados": [finding("x.rs", 1, "bloqueia")]});
        let outcome = evaluate(Some(&output.to_string()), "", &changed);
        assert_eq!(outcome.verdict, "approve");
        assert_eq!(outcome.discarded, 1);
        assert!(outcome.message().contains("fora do diff"));
    }

    #[test]
    fn text_fallback_and_invalid_output() {
        let changed = changed_lines(DIFF_U0);
        let text = "Segue: {\"veredito\":\"approve\",\"achados\":[]} fim";
        assert_eq!(evaluate(None, text, &changed).verdict, "approve");
        assert_eq!(evaluate(None, "sem json", &changed).verdict, "invalid");
        assert_eq!(
            evaluate(Some(r#"{"veredito":"talvez","achados":[]}"#), "", &changed).verdict,
            "invalid"
        );
    }

    #[test]
    fn review_runs_only_with_risk_unless_configured() {
        assert!(!is_due(Quando::Risco, Verdict::Proved, 10));
        assert!(is_due(Quando::Risco, Verdict::NotProved, 10));
        assert!(is_due(Quando::Risco, Verdict::NoTests, 10));
        assert!(is_due(Quando::Risco, Verdict::Proved, 301));
        assert!(is_due(Quando::Sempre, Verdict::Proved, 1));
        assert!(!is_due(Quando::Manual, Verdict::NotProved, 999));
    }

    #[test]
    fn schema_is_valid_json() {
        let value: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        assert_eq!(value["required"][0], "veredito");
    }
}
