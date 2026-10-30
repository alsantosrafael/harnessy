---
status: superseded by 0003
---

# Canal do agente de volta ao app: `harnessy mv` por socket local

O agente (Claude Code) rodava sem nenhum retorno ao app: só o Rust decidia o estado da run. Para que ele também mova cards (por exemplo, "bloqueado" ou, no futuro, o revisor Codex aprovando), decidimos dar a cada run um canal mínimo, em vez de MCP ou de um token de API.

## Decisão

- Cada run cria um diretório privado (0700) em `$TMPDIR` com um socket Unix (0600), um token aleatório de 128 bits e um shim `bin/harnessy` que reexecuta o executável do app com `--cli`. O shim entra no início do `PATH` do agente; `HARNESSY_SOCKET` e `HARNESSY_TOKEN` entram no ambiente fixo.
- Protocolo de uma linha (`token\top\targ\tnota`) e resposta `ok` ou `err <código>`. Operações: `mv` (mover a tarefa) e `note` (anotar no histórico). Não há leitura: o agente não consulta tarefas, board nem histórico.
- O servidor só atende com o token da run e só altera a tarefa dessa run. Cada mudança entra no histórico com ator `agent`; notas passam por `redact::clean` e têm 280 bytes no máximo.
- A permissão de Bash do agente ganha exatamente `harnessy mv *` e `harnessy note *`. `--strict-mcp-config` continua sem servidor MCP; `gh`, `git push` e rede seguem negados.
- Ao fim da run o servidor para e o diretório é apagado.

## Precedência entre a run e o agente

A falha da run vence a palavra do agente: um `review` de uma run que falhou, foi cancelada ou interrompida volta para `todo`. Já um movimento do agente para outro estado (por exemplo, `todo` com "bloqueado") vence uma run que terminou bem. Nada reabre uma tarefa `done`. A regra vive em `flow::sync::reconcile`.

## Por que assim

- **Custo de tokens.** O caminho feliz não gasta nada: o Rust já move o card pelo estado da run. O comando só aparece quando o agente foge disso, com tarefa implícita (sem id), saída `ok` e duas linhas no prompt.
- **Sem MCP.** Um servidor MCP exigiria relaxar `--strict-mcp-config` e carregaria definições de ferramentas no contexto de toda run.
- **Sem sidecar.** O próprio executável do app serve de cliente, então não há binário extra para empacotar.

## Risco aceito

Qualquer processo do mesmo usuário que descubra o caminho do socket e o token pode mover a tarefa da run enquanto ela durar. O token vive só no ambiente do agente e no diretório 0700; o dano máximo é mudar o estado de uma tarefa, com registro no histórico e desfazer em um clique. Isso não amplia o risco do ADR 0001, que já aceita que um agente comprometido executa código no host.
