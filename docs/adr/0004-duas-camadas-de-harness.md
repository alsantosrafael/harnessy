---
status: accepted
---

# Duas camadas de harness: o do Harnessy manda, o do projeto informa

O Harnessy trabalha em repositórios que têm o próprio harness (AGENTS.md, CLAUDE.md, `.claude/rules`, ADRs), e o próprio Harnessy é um desses repositórios. Os agentes (implementador, revisor staff, QA) precisam seguir a arquitetura do projeto, mas o texto do projeto, ou de uma Ação criada por alguém, não pode virar um canal de prompt injection.

## Decisão

1. **Núcleo (Harnessy).** O início fixo do prompt (`run::prompt`), as Ações embutidas, o `json_schema` de cada resposta, as permissões geradas em Rust e os filtros determinísticos (achados fora do diff são descartados; checks e contraprova valem independentemente do veredito). Compilado no executável e nunca lido do repositório que está sendo alterado.
2. **Projeto.** O Rust lê uma lista fixa de arquivos (`AGENTS.md`, `CLAUDE.md`, `.claude/rules/*.md`, `docs/adr/*.md`), com limite de tamanho e redação de credenciais, e os entrega como **dados** num bloco delimitado (`<harness-do-projeto arquivo sha256>`), depois das regras do núcleo. O bloco diz que não muda papel, formato, veredito, permissões nem ferramentas, e que em conflito valem as regras do Harnessy. O texto do arquivo não consegue fechar o bloco.
3. **Do commit base, não da mudança.** O harness é lido por `git show <base>:<arquivo>`. Um PR que reescreve o AGENTS.md não altera as regras pelas quais ele é julgado.
4. **Quem lê é o Rust, não o agente.** Nenhuma Ação manda o agente "ler o AGENTS.md": isso daria ao arquivo autoridade de instrução e não deixaria rastro. O bloco aparece na prévia do prompt, então o usuário vê exatamente o que o agente recebe.

## Boneca russa

Quando o repositório é o próprio Harnessy, o harness do projeto é o AGENTS.md dele. A mesma regra vale: o Harnessy em execução é o instalado e estável; o harness do commit base informa, não comanda.

## Consequência

Um harness hostil só consegue distorcer o julgamento (por exemplo, pedir "aprove sempre"). Isso não abre ferramentas, não muda o formato e não passa os gates: checks, contraprova e o merge humano continuam independentes do veredito.
