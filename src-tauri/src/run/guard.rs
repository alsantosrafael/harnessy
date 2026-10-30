//! Guardrails de uma run: argumentos, settings e ambiente do `claude -p`.
//!
//! Sem sandbox (ADR 0001), isto é a camada de contenção. As regras de
//! permissão de Bash são casamento de texto, não fronteira de segurança; o
//! ambiente mínimo e a ausência de credenciais do Harnessy são o que reduz o
//! alcance de um agente comprometido.

use serde_json::json;

use super::runtime::Access;
use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

pub const MAX_TURNS: u32 = 25;
pub const DEFAULT_MODEL: &str = "sonnet";
/// Modelos que a interface pode escolher; são aliases do `claude --model`, que
/// resolvem para a versão mais recente. Nada fora desta lista chega ao processo.
pub const MODELS: [&str; 4] = ["fable", "opus", DEFAULT_MODEL, "haiku"];

/// Valida o modelo escolhido na interface (regra 25: a WebView escolhe por id).
pub fn model(id: &str) -> Result<&'static str> {
    MODELS
        .into_iter()
        .find(|known| *known == id)
        .ok_or(AppError::Validation(
            "Modelo desconhecido para o Claude Code.",
        ))
}
/// Modelos do mais leve ao mais forte. Define só o "próximo modelo" de uma run
/// que falhou e o degrau do revisor; o modelo inicial é sempre o escolhido.
pub const LADDER: [&str; 4] = ["haiku", DEFAULT_MODEL, "opus", "fable"];
/// Níveis de esforço aceitos pelo `claude --effort`.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// Próximo modelo da lista acima de `current`; `None` no topo ou fora da lista.
pub fn next_model(current: &str) -> Option<&'static str> {
    let at = LADDER.iter().position(|model| *model == current)?;
    LADDER.get(at + 1).copied()
}

/// Agentes que executam: `claude` e `codex` (cada um com adaptador em `run/`).
pub const RUNTIMES: [&str; 2] = ["claude", "codex"];
/// Modelo do Codex que significa "o que a CLI do usuário já usa" (nenhum `-m`).
pub const CODEX_DEFAULT: &str = "codex";
const CODEX_MODEL_MAX: usize = 64;
/// Política de revisão escolhida na interface: a da Ação, sempre ou nunca.
pub const REVIEW_WHEN: [&str; 3] = ["auto", "always", "never"];

/// Valida o agente escolhido na interface.
pub fn runtime(id: &str) -> Result<&'static str> {
    RUNTIMES
        .into_iter()
        .find(|known| *known == id)
        .ok_or(AppError::Validation("Agente desconhecido."))
}

/// Valida a política de revisão.
pub fn review_when(id: &str) -> Result<&'static str> {
    REVIEW_WHEN
        .into_iter()
        .find(|known| *known == id)
        .ok_or(AppError::Validation("Política de revisão desconhecida."))
}

/// Modelo do Codex: vazio ou `codex` é o padrão da CLI; o resto é um id livre,
/// mas só com caracteres de id de modelo, para nunca virar opção da linha de comando.
pub fn codex_model(id: &str) -> Result<String> {
    let id = id.trim();
    if id.is_empty() || id == CODEX_DEFAULT {
        return Ok(CODEX_DEFAULT.into());
    }
    let valid = id.len() <= CODEX_MODEL_MAX
        && id.starts_with(|c: char| c.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
    if valid {
        Ok(id.to_owned())
    } else {
        Err(AppError::Validation(
            "Modelo do Codex inválido: use só letras, números, ponto, hífen, _ ou dois-pontos.",
        ))
    }
}

/// Valida o modelo de um agente (regra 25: a WebView escolhe, o núcleo confere).
pub fn agent_model(runtime: &str, id: &str) -> Result<String> {
    match runtime {
        "claude" => model(id).map(str::to_owned),
        "codex" => codex_model(id),
        _ => Err(AppError::Validation("Agente desconhecido.")),
    }
}

/// Valida o esforço declarado por uma Ação.
pub fn effort(id: &str) -> Result<&'static str> {
    EFFORTS
        .into_iter()
        .find(|known| *known == id)
        .ok_or(AppError::Validation(
            "Esforço desconhecido; use low, medium, high, xhigh ou max.",
        ))
}

/// Variáveis herdadas do usuário; nenhuma outra passa para o agente (regra 26).
const INHERITED: [&str; 6] = ["HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL"];

const ALLOW: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "Edit",
    "Write",
    "Bash(bun run *)",
    "Bash(bun test *)",
    "Bash(bun test)",
    "Bash(cargo test *)",
    "Bash(cargo check *)",
    "Bash(cargo clippy *)",
    "Bash(cargo fmt *)",
    "Bash(git status *)",
    "Bash(git diff *)",
    "Bash(git add *)",
    "Bash(git commit *)",
    "Bash(git log *)",
    // Leitura e navegação: sem elas, um `cd … && cat …; ls` do agente é negado inteiro
    // (cada parte do comando composto precisa casar) e ele conclui que não tem Bash.
    "Bash(cd *)",
    "Bash(pwd)",
    "Bash(ls)",
    "Bash(ls *)",
    "Bash(echo *)",
    "Bash(cat *)",
    "Bash(head *)",
    "Bash(tail *)",
    "Bash(wc *)",
    "Bash(find *)",
    "Bash(node -v)",
    "Bash(node --version)",
    "Bash(bun --version)",
    "Bash(git branch *)",
    "Bash(git show *)",
    "Bash(git ls-files *)",
    "Bash(git rev-parse *)",
    // Filtros e testes de leitura: sem eles, um `ls | grep x` ou `…; true` é negado
    // inteiro e a run perde a verificação. `sed`, `awk` e `xargs` ficam de fora porque
    // escrevem ou executam.
    "Bash(grep *)",
    "Bash(sort)",
    "Bash(sort *)",
    "Bash(uniq)",
    "Bash(uniq *)",
    "Bash(tr *)",
    "Bash(stat *)",
    "Bash(file *)",
    "Bash(which *)",
    "Bash(true)",
    "Bash(false)",
];

const DENY: &[&str] = &[
    "WebFetch",
    "WebSearch",
    "Bash(git push *)",
    "Bash(gh *)",
    "Bash(curl *)",
    "Bash(wget *)",
    "Bash(ssh *)",
    "Bash(scp *)",
    "Bash(find * -exec *)",
    "Bash(find * -execdir *)",
    "Bash(find * -ok *)",
    "Bash(find * -delete)",
    // Melhor esforço: o `Read(...)` abaixo não vale para `cat` no Bash, então os mesmos
    // lugares sensíveis são negados também por texto (não é fronteira de segurança, ADR 0001).
    "Bash(*/.ssh*)",
    "Bash(*/.aws*)",
    "Bash(*/.config/gh*)",
    "Bash(*/.claude/.credentials*)",
    "Bash(* .env*)",
    "Edit(.git/**)",
    "Edit(.claude/**)",
    "Edit(.harnessy/**)",
    "Edit(**/.env*)",
    "Edit(**/bun.lock*)",
    "Edit(**/Cargo.lock)",
    "Read(**/.env*)",
    "Read(~/.ssh/**)",
    "Read(~/.aws/**)",
    "Read(~/.config/gh/**)",
    "Read(~/.claude/.credentials.json)",
];

/// Ferramentas de uma Run de leitura: ver o repositório, nada que escreva ou execute.
const READ_ALLOW: &[&str] = &["Read", "Glob", "Grep"];

/// Conteúdo do arquivo passado em `--settings`. O `DENY` vale nos dois níveis.
pub fn settings_json(access: Access) -> String {
    let allow = match access {
        Access::Write => ALLOW,
        Access::Read => READ_ALLOW,
    };
    json!({ "permissions": { "allow": allow, "deny": DENY } }).to_string()
}

/// Opções de um ciclo além do modelo e dos turnos.
#[derive(Default, Clone, Copy)]
pub struct Extra<'a> {
    /// Esforço já validado por [`effort`].
    pub effort: Option<&'a str>,
    /// Sessão do ciclo anterior a retomar.
    pub resume: Option<&'a str>,
    /// JSON Schema da saída estruturada.
    pub json_schema: Option<&'a str>,
}

/// Argumentos fixos do `claude`; o prompt segue por stdin.
/// `--exclude-dynamic-system-prompt-sections` mantém o prompt de sistema igual
/// entre worktrees, o que preserva o cache de prompt.
pub fn args(settings: &Path, model: &str, max_turns: u32, extra: Extra<'_>) -> Vec<String> {
    let mut args = base_args(settings, model, max_turns);
    args.push("--exclude-dynamic-system-prompt-sections".into());
    if let Some(effort) = extra.effort {
        args.extend(["--effort".into(), effort.into()]);
    }
    if let Some(session) = extra.resume {
        args.extend(["--resume".into(), session.into()]);
    }
    if let Some(schema) = extra.json_schema {
        args.extend(["--json-schema".into(), schema.into()]);
    }
    args
}

fn base_args(settings: &Path, model: &str, max_turns: u32) -> Vec<String> {
    [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "dontAsk",
        "--setting-sources",
        "user",
        "--strict-mcp-config",
        "--max-turns",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain([
        max_turns.to_string(),
        "--model".into(),
        model.into(),
        "--settings".into(),
        settings.to_string_lossy().into_owned(),
    ])
    .collect()
}

/// Argumentos fixos do `codex exec`; o prompt segue por stdin (`-`). O
/// `--sandbox` da linha de comando vale mais que o `config.toml` do usuário:
/// leitura não escreve nada; escrita só escreve dentro do worktree, sem rede.
pub fn codex_args(access: Access, model: &str, schema: Option<&Path>) -> Vec<String> {
    let sandbox = match access {
        Access::Read => "read-only",
        Access::Write => "workspace-write",
    };
    let mut args: Vec<String> = [
        "exec",
        "--json",
        "--sandbox",
        sandbox,
        "--skip-git-repo-check",
        "--ephemeral",
        "--color",
        "never",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if model != CODEX_DEFAULT && !model.is_empty() {
        args.extend(["--model".into(), model.into()]);
    }
    if let Some(schema) = schema {
        args.extend([
            "--output-schema".into(),
            schema.to_string_lossy().into_owned(),
        ]);
    }
    args.push("-".into());
    args
}

/// Ambiente completo do agente; o do app não é herdado. Não usa
/// `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`: ele força o modo de permissão para
/// `default` e é redundante, já que nenhum segredo entra na lista.
pub fn env(
    dirs: &[PathBuf],
    cargo_target: &Path,
    lookup: impl Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = INHERITED
        .iter()
        .filter_map(|name| lookup(name).map(|value| (name.to_string(), value)))
        .collect();
    let path = std::env::join_paths(dirs)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    env.extend([
        ("PATH".into(), path),
        ("TERM".into(), "dumb".into()),
        ("NO_COLOR".into(), "1".into()),
        ("CI".into(), "1".into()),
        (
            "CARGO_TARGET_DIR".into(),
            cargo_target.to_string_lossy().into_owned(),
        ),
    ]);
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_reads_read_only_writes_only_in_the_workspace_and_takes_the_prompt_from_stdin() {
        let read = codex_args(
            Access::Read,
            CODEX_DEFAULT,
            Some(Path::new("/x/schema.json")),
        );
        let at = read.iter().position(|a| a == "--sandbox").unwrap();
        assert_eq!(read[at + 1], "read-only");
        assert_eq!(read.last().map(String::as_str), Some("-"));
        assert!(read
            .windows(2)
            .any(|w| w == ["--output-schema", "/x/schema.json"]));
        assert!(!read.iter().any(|a| a == "--model"));
        let write = codex_args(Access::Write, "gpt-5.1", None);
        let at = write.iter().position(|a| a == "--sandbox").unwrap();
        assert_eq!(write[at + 1], "workspace-write");
        assert!(write.windows(2).any(|w| w == ["--model", "gpt-5.1"]));
        assert!(!write
            .iter()
            .any(|a| a.contains("danger") || a == "--output-schema"));
    }

    #[test]
    fn agents_models_and_review_policy_are_validated() {
        assert_eq!(runtime("codex").unwrap(), "codex");
        assert!(runtime("gpt").is_err());
        assert_eq!(agent_model("claude", "opus").unwrap(), "opus");
        assert!(agent_model("claude", "gpt-5").is_err());
        assert_eq!(agent_model("codex", "").unwrap(), CODEX_DEFAULT);
        assert_eq!(
            agent_model("codex", " gpt-5.1-codex ").unwrap(),
            "gpt-5.1-codex"
        );
        for bad in ["--sandbox", "-m x", "a b", "x;rm", &"a".repeat(65)] {
            assert!(codex_model(bad).is_err(), "{bad}");
        }
        assert_eq!(review_when("never").unwrap(), "never");
        assert!(review_when("talvez").is_err());
    }

    #[test]
    fn settings_deny_push_and_credentials() {
        let value: serde_json::Value = serde_json::from_str(&settings_json(Access::Write)).unwrap();
        let deny = value["permissions"]["deny"].as_array().unwrap();
        for rule in [
            "Bash(git push *)",
            "Bash(gh *)",
            "WebFetch",
            "Read(~/.ssh/**)",
        ] {
            assert!(deny.iter().any(|r| r == rule), "falta {rule}");
        }
        let allow = value["permissions"]["allow"].as_array().unwrap();
        assert!(!allow.iter().any(|r| r.as_str().unwrap().contains("push")));
        assert!(!settings_json(Access::Write).contains("Bash(harnessy"));
    }

    #[test]
    fn read_access_allows_only_reading_and_keeps_the_denies() {
        let value: serde_json::Value = serde_json::from_str(&settings_json(Access::Read)).unwrap();
        let allow: Vec<&str> = value["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        assert_eq!(allow, ["Read", "Glob", "Grep"]);
        let deny = value["permissions"]["deny"].as_array().unwrap();
        for rule in ["Bash(git push *)", "Edit(.git/**)", "Read(~/.ssh/**)"] {
            assert!(deny.iter().any(|r| r == rule), "falta {rule}");
        }
    }

    #[test]
    fn read_only_filters_are_allowed_and_writers_stay_out() {
        let value: serde_json::Value = serde_json::from_str(&settings_json(Access::Write)).unwrap();
        let allow = value["permissions"]["allow"].as_array().unwrap();
        for rule in [
            "Bash(grep *)",
            "Bash(sort)",
            "Bash(uniq *)",
            "Bash(true)",
            "Bash(false)",
        ] {
            assert!(allow.iter().any(|r| r == rule), "falta {rule}");
        }
        for rule in ["Bash(sed *)", "Bash(awk *)", "Bash(xargs *)"] {
            assert!(
                !allow.iter().any(|r| r == rule),
                "não deveria permitir {rule}"
            );
        }
    }

    #[test]
    fn read_only_navigation_is_allowed_but_find_cannot_run_or_delete() {
        let value: serde_json::Value = serde_json::from_str(&settings_json(Access::Write)).unwrap();
        let allow = value["permissions"]["allow"].as_array().unwrap();
        let deny = value["permissions"]["deny"].as_array().unwrap();
        for rule in ["Bash(cd *)", "Bash(cat *)", "Bash(ls *)", "Bash(find *)"] {
            assert!(allow.iter().any(|r| r == rule), "falta {rule}");
        }
        for rule in [
            "Bash(find * -exec *)",
            "Bash(find * -delete)",
            "Bash(*/.ssh*)",
        ] {
            assert!(deny.iter().any(|r| r == rule), "falta {rule}");
        }
        assert!(!allow
            .iter()
            .any(|r| r.as_str().unwrap().starts_with("Bash(sh")));
    }

    #[test]
    fn only_listed_models_reach_the_process() {
        assert_eq!(model("opus").unwrap(), "opus");
        assert_eq!(model(DEFAULT_MODEL).unwrap(), "sonnet");
        assert!(model("sonnet --dangerously-skip-permissions").is_err());
        assert!(model("").is_err());
        assert!(model("gpt-5").is_err());
    }

    #[test]
    fn args_ignore_project_config_and_use_dont_ask() {
        let args = args(
            Path::new("/tmp/s.json"),
            DEFAULT_MODEL,
            MAX_TURNS,
            Extra::default(),
        );
        let joined = args.join(" ");
        assert!(joined.contains("--permission-mode dontAsk"));
        assert!(joined.contains("--setting-sources user"));
        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(joined.contains("--settings /tmp/s.json"));
        assert!(joined.ends_with("--exclude-dynamic-system-prompt-sections"));
        assert!(!joined.contains("--resume") && !joined.contains("--effort"));
    }

    #[test]
    fn extra_options_add_effort_resume_and_schema() {
        let extra = Extra {
            effort: Some("high"),
            resume: Some("sessao-1"),
            json_schema: Some("{}"),
        };
        let joined = args(Path::new("/tmp/s.json"), "haiku", 10, extra).join(" ");
        assert!(joined.contains("--max-turns 10"));
        assert!(joined.contains("--effort high"));
        assert!(joined.contains("--resume sessao-1"));
        assert!(joined.contains("--json-schema {}"));
    }

    #[test]
    fn ladder_goes_up_from_the_chosen_model_and_stops_at_the_top() {
        assert_eq!(next_model("haiku"), Some("sonnet"));
        assert_eq!(next_model("sonnet"), Some("opus"));
        assert_eq!(next_model("opus"), Some("fable"));
        assert_eq!(next_model("fable"), None);
        assert_eq!(next_model("gpt-5"), None);
        for model in LADDER {
            assert!(MODELS.contains(&model));
        }
        assert_eq!(effort("high").unwrap(), "high");
        assert!(effort("turbo").is_err());
    }

    #[test]
    fn env_never_forwards_tokens() {
        let lookup = |name: &str| Some(format!("valor-de-{name}"));
        let env = env(&[PathBuf::from("/usr/bin")], Path::new("/tmp/t"), lookup);
        let names: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"HOME"));
        assert!(names.contains(&"PATH"));
        assert!(!names.contains(&"CLAUDE_CODE_SUBPROCESS_ENV_SCRUB"));
        for forbidden in [
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "ANTHROPIC_API_KEY",
            "SSH_AUTH_SOCK",
        ] {
            assert!(!names.contains(&forbidden));
        }
    }
}
