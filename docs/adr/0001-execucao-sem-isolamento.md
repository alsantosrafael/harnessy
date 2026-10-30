---
status: accepted
---

# Runs de agente sem isolamento de processo, só em workspaces confiáveis

O SPEC §13 exige sandbox real para executar agentes. Decidimos começar sem sandbox nem contêiner (`isolation: none`), restrito a Workspaces que o usuário marca como confiáveis (primeiro caso: o próprio Harnessy), porque o objetivo imediato é usar o fluxo Tarefa → Run → PR. Sandbox, contêiner e token guardado no Keychain ficam adiados para projetos que precisem, como deploy de site; o campo `isolation` do Workspace já reserva o valor `sandbox`.

## Controles que valem sem isolamento

- O Harnessy não guarda credenciais; o agente usa o login que o usuário já fez na CLI, com `HOME` real.
- Ambiente do processo por lista fixa em Rust, sem `GH_TOKEN` nem outros segredos. `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` não é usado: ele força o modo de permissão para `default`.
- `--permission-mode dontAsk` com lista de permissões gerada por Run (inclui leitura e navegação como `cd`, `ls`, `cat`, `find`, porque um comando composto é negado inteiro se uma parte não casar; `find -exec/-delete` e caminhos sensíveis são negados por texto, como melhor esforço); `git push`, `gh`, `curl`, `ssh` e acesso web negados; edição de `.git`, `.claude`, `.harnessy`, `.env*` e lockfiles negada.
- Settings e MCP do projeto ignorados (`--setting-sources user`, `--strict-mcp-config`).
- Worktree criado a partir de arquivos commitados, então `.env` do repositório não é copiado.
- Push e PR são feitos pelo Rust, depois de verificar o diff; nenhum merge automático.
- Limites baixos de tempo e turnos por Run.

## Risco aceito

As regras de permissão de Bash são casamento de texto, não fronteira de segurança: `sh -c`, caminhos absolutos ou um script que abre arquivos por conta própria podem escapar delas. Um agente comprometido, por exemplo por prompt injection em um arquivo do repositório ou em uma tarefa colada de fora, ainda pode ler arquivos fora do worktree (como `~/.ssh`) e executar código no host. A mitigação é reduzir a entrada não confiável (as tarefas são só locais, escritas por você, com o prompt exato mostrado antes da Run) e revisar todo PR antes do merge. Este ADR deve ser revisto antes de usar o Harnessy em qualquer Workspace não confiável.

## Verificação (spike, 2026-10-29)

Uma run real de `claude -p` com o `--settings` gerado, em repositório temporário, confirmou: `git push`, `gh`, `curl` e leitura em `~/.ssh` negados; `git status` permitido; nenhum MCP carregado; hook de `.claude/settings.json` do projeto não executado; modo `dontAsk` efetivo. `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` foi retirado porque força o modo `default`. Não foi testado um subprocesso que leia arquivos por conta própria (por exemplo um script), que as regras de permissão não cobrem.
