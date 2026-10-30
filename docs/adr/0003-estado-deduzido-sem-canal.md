---
status: accepted
---

# Estado do card deduzido do resultado da run, sem canal do agente

O [ADR 0002](0002-canal-do-agente.md) deu ao agente um comando (`harnessy mv`/`note`) para mover o card. Ele era barato, mas não era de graça: duas linhas no prompt de toda run, um turno extra a cada uso e uma superfície privilegiada (socket, token, shim, `--cli`). O objetivo do Harnessy é digitar o mínimo e gastar o mínimo, então o card passa a ser gerido com **custo zero de tokens**: o agente não sabe que existe card.

## Decisão

O Harnessy deduz o estado do card e o bloqueio de fatos que já tem:

| Estado da run | Card vai para | Nota no histórico (ator `system`) | Bloqueio da run |
| --- | --- | --- | --- |
| `preparing`, `implementing` | Em andamento | — | — |
| `completed` | Revisão | resumo final do agente | só se o `claude` negou ações |
| `no_changes` | A fazer | "Sem alterações: " + resumo do agente | sim, o mesmo texto |
| `failed` | A fazer | "Run falhou: " + motivo | sim, o motivo (+ negações) |
| `cancelled` | A fazer | "Run cancelada." | não |
| `interrupted` (abertura do app) | A fazer | "Run interrompida: o Harnessy foi fechado." | não |

- O resumo final que o agente já escreve vira a nota do histórico. O prompt não ganha uma palavra.
- **Bloqueio** mora na run (`runs.blocked`, texto; vazio = sem bloqueio). Sinais: falha da run, run que terminou sem mudar nada e `permission_denials` do resultado do `claude` (cada negação vira um evento `denied` no log). O front mostra o selo "Bloqueada" quando a última run da tarefa terminou com bloqueio e a tarefa não está Concluída.
- O usuário resolve o bloqueio por `resolve_block`, que limpa o texto e registra "Bloqueio resolvido." no histórico.
- Nunca se reabre uma tarefa Concluída. A regra vive em `flow::sync` (`blocker`, `reconcile`, `recover`, `resolve`).

## Risco aceito

Se o agente muda código e está bloqueado só por um motivo semântico (por exemplo, uma ambiguidade), o card vai para Revisão com o resumo dele no histórico, sem o selo. Adivinhar bloqueio pelo texto do agente (por exemplo, "termina com `?`") foi descartado por gerar falso positivo quando ele só oferece algo ao fim, e `--json-schema` põe o esquema no contexto, o que custa token. O revisor Codex, quando existir, devolve o veredito no texto final lido pelo Rust; `reconcile` é o ponto único onde ele entra.
