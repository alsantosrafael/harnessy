//! Início fixo do prompt de uma Run. O núcleo monta as regras que valem para
//! toda Ação; a Ação fornece só o pedido final, já preenchido (regras 25 e 27).
//! Uma Ação não consegue remover estas regras, porque o início não vem dela.

use super::runtime::Access;

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
                "O Harnessy roda as verificações do projeto ao final; você pode rodar os comandos de verificação antes se quiser conferir.",
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
            "O Harnessy roda as verificações do projeto ao final; você pode rodar os comandos de verificação antes se quiser conferir."
        ));
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
}
