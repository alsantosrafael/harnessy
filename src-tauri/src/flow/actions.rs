//! Ações na camada de composição: carrega o catálogo com a validação de modelo,
//! que o domínio `action` não pode fazer (regra 4), e monta o prompt de uma Run.

use std::path::Path;

use crate::{
    action::{self, Acesso, Action, Contexto, Entrada, Repositorio},
    error::{AppError, Result},
    run::{guard, prompt, runtime::Access},
};

/// Todas as Ações disponíveis (embutidas, do usuário e, com `repo`, do
/// repositório), com modelo, esforço e turnos já validados.
pub fn load(data_dir: &Path, repo: Option<&Repositorio<'_>>) -> Vec<Entrada> {
    let mut entradas = action::carregar(&action::user_dir(data_dir), repo);
    for entrada in &mut entradas {
        let erro = match &entrada.acao {
            Ok(acao) => validate(acao).err(),
            Err(_) => None,
        };
        if let Some(erro) = erro {
            entrada.acao = Err(AppError::Action(erro.to_string()));
        }
    }
    entradas
}

/// Confere o que depende do runtime: modelo, esforço e teto de turnos.
fn validate(acao: &Action) -> Result<()> {
    if let Some(modelo) = acao.modelo.as_deref() {
        guard::model(modelo)?;
    }
    if let Some(esforco) = acao.esforco.as_deref() {
        guard::effort(esforco)?;
    }
    if acao.max_turns.is_some_and(|turns| turns > guard::MAX_TURNS) {
        return Err(AppError::Validation(
            "max_turns passa do limite do Harnessy (25).",
        ));
    }
    Ok(())
}

/// Teto de turnos de uma Ação, nunca acima do limite do núcleo.
pub fn max_turns(action: &Action) -> u32 {
    action
        .max_turns
        .unwrap_or(guard::MAX_TURNS)
        .min(guard::MAX_TURNS)
}

/// Uma Ação pelo id. Falha se o id não existe ou se a Ação está inválida.
pub fn find(data_dir: &Path, repo: Option<&Repositorio<'_>>, id: &str) -> Result<Action> {
    let entrada = load(data_dir, repo)
        .into_iter()
        .find(|entrada| entrada.id == id)
        .ok_or(AppError::Validation("Ação não encontrada."))?;
    entrada.acao
}

/// Ids das Ações que só leem. Usado na recuperação de runs para não mover o card.
pub fn read_only_ids(data_dir: &Path) -> Vec<String> {
    load(data_dir, None)
        .into_iter()
        .filter(|entrada| matches!(&entrada.acao, Ok(acao) if acao.acesso == Acesso::Leitura))
        .map(|entrada| entrada.id)
        .collect()
}

pub fn run_access(acesso: Acesso) -> Access {
    match acesso {
        Acesso::Leitura => Access::Read,
        Acesso::Escrita => Access::Write,
    }
}

/// Prompt completo de uma Run: início fixo do núcleo, comandos que o Harnessy já
/// negou neste workspace e o pedido da Ação preenchido.
pub fn build_prompt(
    action: &Action,
    ctx: &Contexto<'_>,
    avoid: &[String],
    project: &str,
) -> String {
    prompt::compose(
        run_access(action.acesso),
        &action.render(ctx),
        avoid,
        project,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::test_support::TempDir;

    fn contexto<'a>(descricao: &'a str, criterios: &'a str) -> Contexto<'a> {
        Contexto {
            tarefa: "Ajustar texto",
            descricao,
            criterios,
            ..Default::default()
        }
    }

    #[test]
    fn implementar_keeps_the_core_prefix_and_fills_the_task() {
        let temp = TempDir::new("implementar");
        let action = find(&temp.path, None, action::IMPLEMENT).unwrap();
        let prompt = build_prompt(
            &action,
            &contexto("Trocar o rótulo.", "O rótulo novo aparece."),
            &[],
            "",
        );
        assert!(prompt.starts_with("Você trabalha neste repositório"));
        assert!(prompt.contains("# Tarefa: Ajustar texto"));
        assert!(prompt.contains("Trocar o rótulo."));
        assert!(prompt.contains("O rótulo novo aparece."));
        assert!(prompt.contains(
            "O Harnessy já instalou as dependências e roda as verificações do projeto ao final"
        ));
        assert!(!prompt.contains("Rode as verificações do projeto descritas no AGENTS.md"));
    }

    #[test]
    fn explicar_is_read_only_and_listed_as_such() {
        let temp = TempDir::new("explicar");
        let action = find(&temp.path, None, action::EXPLAIN).unwrap();
        assert_eq!(run_access(action.acesso), Access::Read);
        let prompt = build_prompt(&action, &contexto("", ""), &[], "");
        assert!(prompt.contains("só de leitura"));
        assert!(prompt.contains("Explique onde e como fazer a mudança"));
        assert_eq!(
            read_only_ids(&temp.path),
            [
                action::EXPLAIN,
                action::REVIEW,
                action::SPLIT,
                action::REFINE
            ]
        );
    }

    #[test]
    fn refinar_asks_closed_questions_and_forbids_delivery_process() {
        let temp = TempDir::new("refinar");
        let action = find(&temp.path, None, action::REFINE).unwrap();
        assert_eq!(run_access(action.acesso), Access::Read);
        let prompt = build_prompt(&action, &contexto("", ""), &[], "");
        assert!(prompt.contains("só de leitura"));
        assert!(prompt.contains("sim/não ou opções curtas"));
        assert!(prompt.contains("Nada de processo"));
        assert!(prompt.contains("`pronta`"));
        assert!(prompt.contains("Não escreva uma seção \"Decisões\""));
        assert!(!prompt.contains("{{"));
    }

    #[test]
    fn user_model_is_validated_here_and_unknown_ids_are_reported() {
        let temp = TempDir::new("modelo");
        // O catálogo lê `<dados>/actions/`, a pasta de Ações do usuário.
        let dir = temp.path.join("actions");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("com-modelo.toml"),
            "schema = 1\nid = \"com-modelo\"\nnome = \"Com modelo\"\ndescricao = \"d\"\n\
             papel = \"implementador\"\nacesso = \"escrita\"\nmodelo = \"gpt-5\"\n\
             pedido = \"Faça {{tarefa}}.\"\n",
        )
        .unwrap();
        let entrada = load(&temp.path, None)
            .into_iter()
            .find(|entrada| entrada.id == "com-modelo")
            .unwrap();
        let erro = entrada.acao.as_ref().err().map(|e| e.to_string());
        assert!(erro.unwrap_or_default().contains("Modelo desconhecido"));
        assert!(find(&temp.path, None, "com-modelo").is_err());
        assert!(find(&temp.path, None, "nao-existe").is_err());
    }

    #[test]
    fn effort_and_turns_are_validated_against_the_core_limits() {
        let temp = TempDir::new("esforco");
        let dir = temp.path.join("actions");
        std::fs::create_dir_all(&dir).unwrap();
        let acao = |id: &str, extra: &str| {
            format!(
                "schema = 1\nid = \"{id}\"\nnome = \"N\"\ndescricao = \"d\"\n\
                 papel = \"implementador\"\nacesso = \"escrita\"\n{extra}\
                 pedido = \"Faça {{{{tarefa}}}}.\"\n"
            )
        };
        std::fs::write(
            dir.join("a.toml"),
            acao("boa", "esforco = \"high\"\nmax_turns = 10\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("b.toml"),
            acao("esforco-ruim", "esforco = \"turbo\"\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("c.toml"),
            acao("turnos-demais", "max_turns = 99\n"),
        )
        .unwrap();
        let boa = find(&temp.path, None, "boa").unwrap();
        assert_eq!(boa.esforco.as_deref(), Some("high"));
        assert_eq!(max_turns(&boa), 10);
        assert!(find(&temp.path, None, "esforco-ruim").is_err());
        assert!(find(&temp.path, None, "turnos-demais").is_err());
        let implementar = find(&temp.path, None, action::IMPLEMENT).unwrap();
        assert_eq!(max_turns(&implementar), guard::MAX_TURNS);
    }
}
