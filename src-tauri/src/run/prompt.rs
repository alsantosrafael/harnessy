//! Início fixo do prompt de uma Run. O núcleo monta as regras que valem para
//! toda Ação; a Ação fornece só o pedido final, já preenchido (regras 25 e 27).
//! Uma Ação não consegue remover estas regras, porque o início não vem dela.

use super::runtime::Access;
use crate::error::{AppError, Result};

/// Tamanho máximo do ajuste pedido numa run encadeada.
const MAX_FOLLOWUP_CHARS: usize = 4_000;

/// Início do prompt, antes do pedido da Ação. Escrita e leitura têm regras próprias.
/// `avoid` são comandos que o Harnessy já negou neste workspace (fato, não opinião).
pub fn compose(access: Access, pedido: &str, avoid: &[String]) -> String {
    let (intro, regras): (&str, &[&str]) = match access {
        Access::Write => (
            "Você trabalha neste repositório. O diretório atual é um worktree git dedicado à Run.",
            &[
                "Siga o AGENTS.md e as convenções do repositório; faça só o que o pedido pede.",
                "Não faça push nem abra PR: o Harnessy publica. Commits locais são permitidos; o que sobrar sem commit o Harnessy commita.",
                "Não edite arquivos protegidos: `.git`, `.claude`, `.harnessy`, `.env*`, `bun.lock` e `Cargo.lock`.",
                "O Harnessy já instalou as dependências e roda as verificações do projeto ao final. Para conferir antes, rode um comando por vez, como `bun run test`, `bun run lint` ou `bun run check`; não rode instalação, `bunx` nem `cd`/`;`/`||` encadeando comandos.",
                "Cada parte de um comando precisa ser permitida: se uma for negada, o comando inteiro falha, mas isso não desliga o Bash. Rode de novo só o comando simples. Edite arquivos com as ferramentas de edição, não com redirecionamento do shell (`>`, `<<EOF`).",
                "Termine com um resumo curto do que mudou.",
            ],
        ),
        Access::Read => (
            "Você responde sobre este repositório. O diretório atual é o repositório da Tarefa.",
            &[
                "Siga o AGENTS.md e as convenções do repositório.",
                "Não edite arquivos e não tente alterar o repositório: esta Ação é só de leitura.",
                "Não faça push nem abra PR.",
                "Termine com um resumo curto da sua resposta.",
            ],
        ),
    };
    let mut prompt = format!("{intro}\n\n## Regras\n");
    for regra in regras {
        prompt.push_str("- ");
        prompt.push_str(regra);
        prompt.push('\n');
    }
    if !avoid.is_empty() {
        let list: Vec<String> = avoid.iter().map(|command| format!("`{command}`")).collect();
        prompt.push_str(&format!(
            "- Não tente: {} (negados pelo Harnessy neste projeto).\n",
            list.join(", ")
        ));
    }
    prompt.push('\n');
    prompt.push_str(pedido);
    prompt
}

/// Ajuste pedido pelo usuário numa run encadeada: vai depois do pedido da Ação,
/// que continua valendo. Texto vazio deixa o prompt como está.
pub fn with_followup(prompt: String, instruction: &str) -> Result<String> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Ok(prompt);
    }
    if instruction.chars().count() > MAX_FOLLOWUP_CHARS {
        return Err(AppError::Validation("O ajuste pedido é longo demais."));
    }
    Ok(format!(
        "{prompt}\n\n## Ajuste pedido agora\nO trabalho anterior desta tarefa já está neste worktree. Faça só o ajuste abaixo, sem refazer o que já está certo.\n\n{instruction}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_prompt_starts_with_the_core_rules_and_drops_the_old_instruction() {
        let prompt = compose(Access::Write, "# Tarefa: Ajustar texto\n", &[]);
        assert!(prompt.starts_with("Você trabalha neste repositório"));
        assert!(prompt.contains("Não faça push nem abra PR"));
        assert!(prompt.contains("Não edite arquivos protegidos"));
        assert!(prompt.contains(
            "O Harnessy já instalou as dependências e roda as verificações do projeto ao final."
        ));
        assert!(prompt.contains("`bun run test`"));
        assert!(prompt.contains("isso não desliga o Bash"));
        assert!(!prompt.contains("Rode as verificações do projeto descritas no AGENTS.md"));
        assert!(prompt.ends_with("# Tarefa: Ajustar texto\n"));
    }

    #[test]
    fn read_prompt_has_no_check_line_and_forbids_editing() {
        let prompt = compose(Access::Read, "Explique.", &[]);
        assert!(prompt.contains("só de leitura"));
        assert!(!prompt.contains("verificações do projeto"));
        assert!(!prompt.contains("Não tente"));
        assert!(prompt.ends_with("Explique."));
    }

    #[test]
    fn denied_commands_become_a_fixed_rule_before_the_request() {
        let avoid = ["curl".to_owned(), "git push".to_owned()];
        let prompt = compose(Access::Write, "Pedido.", &avoid);
        let rule = "- Não tente: `curl`, `git push` (negados pelo Harnessy neste projeto).";
        assert!(prompt.contains(rule));
        assert!(prompt.find(rule).unwrap() < prompt.find("Pedido.").unwrap());
    }

    #[test]
    fn followup_goes_after_the_request_and_empty_text_changes_nothing() {
        let base = compose(Access::Write, "Pedido.", &[]);
        assert_eq!(with_followup(base.clone(), "  ").unwrap(), base);
        assert!(with_followup(base.clone(), &"x".repeat(4_001)).is_err());
        let prompt = with_followup(base.clone(), "Use vírgula no custo.").unwrap();
        assert!(prompt.starts_with(&base));
        assert!(prompt.ends_with("Use vírgula no custo.\n"));
        assert!(prompt.contains("## Ajuste pedido agora"));
    }
}
