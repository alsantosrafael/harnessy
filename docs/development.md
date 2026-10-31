# Desenvolvimento

[← README](../README.md) · [Instalação](getting-started.md) · [Fundação e validações](foundation.md)

## Ciclo diário

Todos os comandos partem da raiz do projeto:

```bash
bun install --frozen-lockfile
bun run start
```

`bun install --frozen-lockfile` é necessário no primeiro setup e quando as dependências travadas mudam, não a cada alteração de código. O Tauri inicia o Vite automaticamente. Para alterar dependências, use `bun add`/`bun add -d` e versione `bun.lock`. Não mantenha lockfiles de gerenciadores diferentes.

## Comandos disponíveis

Para lint, formatação, ciclos rápido/nativo, CI e auditoria contextual, consulte o [harness de desenvolvimento](harness.md). `bun run check:quick` é o gate frontend diário; `bun run verify` inclui o build.

| Comando | Resultado |
| --- | --- |
| `bun run start` | Desktop Tauri em desenvolvimento com Vite. |
| `bun run dev` | Frontend no navegador em `127.0.0.1:1420`, sem núcleo Rust. |
| `bun run check` | TypeScript e vínculo de versão do produto. |
| `bun run build` | TypeScript e build frontend em `dist/`. |
| `bun run check:rust` | Compilação de verificação Rust, sem bundle. |
| `bun run desktop:build` | Frontend, binário de produção e bundle macOS `.app`. |
| `bun run tauri info` | Diagnóstico do ambiente Tauri. |
| `bun run test` | Testes de regressão com Vitest. |
| `bun run test:watch` | Vitest em modo interativo. |

Verificações adicionais:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Vitest tem configuração própria em `vitest.config.ts`, com ambiente Node e casos em `tests/`. A suíte inicial verifica o contrato de sessão de rascunhos: compatibilidade, corrupção, falha de escrita, retry, sugestões e debounce. Não use `bun test` (runner nativo do Bun); use `bun run test` para executar Vitest. Node 22+ continua necessário para o runner/worker. As sessões Playwright registradas em [foundation.md](foundation.md) complementam a suíte, mas não substituem automação desktop.

## Estrutura

```text
src/
  App.tsx                  # Navegação, layout e telas iniciais
  components/              # Tarefas (kanban), Chat, Conexões e Contexto
  lib/desktop.ts           # Interface tipada para metadados Rust
  lib/drafts.ts            # Sessão de rascunho, recuperação e persistência
  styles/tokens.css        # Primitivos, semânticos e componentes
  styles/app.css           # Estilos da fundação
  styles/refinements.css   # Refinamentos de UX e adaptação
src-tauri/
  src/lib.rs               # Builder e registro de comandos (ver docs/architecture-rust.md)
  src/main.rs              # Entrada do executável
  src/error.rs             # AppError único, serializado como string
  src/db.rs                # Conexão SQLite e migrações
  src/app/                 # app_info, setup, bandeja e janela
  src/workspace/, src/task/, src/run/, src/connections/  # Domínios: regras em mod.rs, IPC em commands.rs
  src/flow/                # Composição: início e pipeline de uma run
  capabilities/main.json   # Permissões da janela: só escutar eventos do núcleo
  tauri.conf.json          # Janela, CSP, build e bundle
  icons/                   # Ícones gerados
scripts/check-version.mjs  # Validação da fonte de versão
tests/drafts.test.ts        # Regressões executadas pelo Vitest
vitest.config.ts            # Ambiente e descoberta dos testes
bun.lock                    # Dependências frontend travadas
docs/                      # Uso, desenvolvimento e pesquisa
```

## Regras da fundação

- Núcleo Rust: estrutura por domínio, erro único e IPC fino conforme [architecture-rust.md](architecture-rust.md); regras obrigatórias em `.claude/rules/rust-architecture.md`.
- Rust controla operações privilegiadas; React não recebe shell ou acesso genérico a arquivos.
- Comandos IPC: `app_info` (metadados), `list_workspaces`/`create_workspace`/`list_tasks`/`save_task`/`move_task`/`task_history` (SQLite local), `check_connections` (estado das CLIs `gh`, `codex` e `claude`), `preview_prompt` (prévia do prompt) e `start_run` (recebe `workspaceId`, `taskId`, o agente e o modelo; só `claude` executa e o modelo precisa estar em `run::guard::MODELS`)/`cancel_run`/`list_runs`/`list_run_events` (runs do agente). A tarefa tem estado `todo → doing → review → done`; toda transição grava um evento de histórico (quem, de → para, nota) na mesma transação e emite `task-changed` (`{taskId, status, revision, event}`). Mover não altera `revision`, que protege só a edição de conteúdo. O Harnessy move o card conforme a run (iniciada → Em andamento, concluída → Revisão, sem alterações/falha/cancelada → A fazer, nunca reabrindo uma tarefa Concluída) e o usuário move por botão, teclado (`]`, `[`, `1`–`4`) ou arrastando. O agente não recebe instrução nem ferramenta de card: o resumo final dele vira a nota do histórico (custo zero de tokens, [ADR 0003](adr/0003-estado-deduzido-sem-canal.md)). `resolve_block` dá o bloqueio de uma run por resolvido. Uma run cria um worktree git, roda `claude -p` com argumentos, settings e ambiente montados em Rust (a WebView só envia ids), e commita na branch `harnessy/run-<id>-<slug>`; não há push nem PR. Progresso chega pelos eventos `run-event`, `run-state` e `task-changed`; a capability `main` concede apenas `core:event:allow-listen` e `core:event:allow-unlisten` (a WebView escuta, não emite). Contenção e risco: [ADR 0001](adr/0001-execucao-sem-isolamento.md). A verificação de conexões executa apenas comandos de status fixos, com entrada fechada e tempo limite; nunca lê, copia ou devolve tokens à WebView.
- Não apresentar integração como funcional sem implementação real. Estados vazios e desabilitados são intencionais.
- A sessão do rascunho vive durante toda a aplicação e só o chat assina suas atualizações; evitar atualização do shell inteiro a cada tecla. Navegar não destrói texto não persistido.
- Usar tokens para aparência e preservar teclado, foco e movimento reduzido.
- Não usar localStorage para credenciais. O rascunho existente é dado de interface, não banco de domínio.
- A fundação não contém sandbox de agentes: restringir capabilities da WebView não comprova isolamento de futuros processos Codex.

## Ações

Cada opção da Tarefa é uma Ação: um TOML embutido (`src-tauri/src/action/embutidas/`: `implementar`, `explicar`, `revisar` e `dividir-em-passos`), um arquivo do usuário em `<dados do app>/actions/*.toml` (válido em todos os workspaces, criado à mão ou pelo formulário **Nova Ação**) ou um arquivo versionado no repositório em `.harnessy/actions/*.toml`.

Campos: `schema = 1`, `id`, `nome`, `descricao`, `papel` (só `implementador`), `acesso` (`leitura` ou `escrita`), `modelo` (opcional; sem ele vale o modelo escolhido), `esforco` (`low`…`max`, repassado como `--effort`), `max_turns` (até 25), `contraprova` (só escrita; padrão ligada), `exigir_prova` (só escrita com contraprova: sem teste que prove a mudança, um ciclo extra pede um), `alvo` (`tarefa` ou `run`), `saida` (`texto`, `revisao` ou `passos`; estruturada só em leitura), `quando` (só em `alvo = "run"`: `risco`, `sempre` ou `manual`) e `pedido`. Variáveis: `{{tarefa}}`, `{{descricao}}`, `{{criterios}}`, `{{arquivos_provaveis}}` (calculado por `git ls-files` + `git grep`) e, só em Ação sobre Run, `{{diff}}`, `{{checks}}` e `{{contraprova}}`. Campo desconhecido, variável desconhecida ou combinação inválida é recusada citando a linha.

Precedência por id: repositório > usuário > embutida. Arquivo inválido aparece em **Mais ações** com o erro e também substitui, para o problema ficar visível. Ação do repositório só roda depois que o usuário confirma o sha256 do arquivo neste computador (`action_approvals`); alterar o arquivo volta a pedir confirmação (SPEC §15). O prompt tem um início fixo do núcleo (`run/prompt.rs`, com a regra "Não tente: …" dos comandos já negados no workspace) e o pedido da Ação no final; a Ação não escolhe ferramentas, ambiente, comandos nem verificação.

- **Leitura:** roda na pasta do workspace com `Read`, `Glob` e `Grep`, sem worktree, checks nem correção, e não move o card. Com `saida = "passos"`, devolve até 5 passos por `--json-schema`; a run mostra a proposta e `create_steps` cria as tarefas encadeadas.
- **Escrita:** worktree, loop de verificação com falha estruturada, retomada de sessão (`--resume`, com recuo para sessão nova), contraprova e revisor.
- **Revisor (`revisar`, `alvo = "run"`):** roda uma vez por run verde; com `quando = "risco"` (padrão), só se a contraprova não provou nada ou o diff passa de 300 linhas. Roda no Claude Code, com um modelo acima do implementador, ou no **Codex** (escolha o revisor, o modelo e **Quando** revisar ao iniciar a run: só com risco, sempre ou nunca; "sempre" vale mais que o `quando` da Ação); recebe só tarefa, critérios, checks, contraprova e diff, e achados fora das linhas alteradas são descartados. Uma Ação do usuário com id `revisar` e `quando = "manual"` desliga a revisão.
- **Revisor Codex (`run/codex.rs`):** `codex exec --json --sandbox read-only --ephemeral`, prompt por stdin e `--output-schema` com o mesmo schema do revisor; o Codex também pode **implementar**: com `--sandbox workspace-write` (só o worktree, sem rede) e sem retomar sessão (a correção leva o contexto completo). O login vem do Codex do usuário (`~/.codex`, ambiente mínimo com `HOME`); o modelo é o da configuração dele ou um id digitado (só letras, números, `.`, `-`, `_`, `:`; validado em `guard::codex_model`); o esforço da Ação não é repassado. A escolha de revisão fica em `runs.reviewer`, `runs.reviewer_model` e `runs.review_when` e vale também para `continue_run`; o evento `review` informa quem revisou. Sem `codex` no PATH, `start_run` recusa antes de criar a run. O ciclo aparece no ledger com o modelo `codex`, sem custo em dólar (a CLI não informa).

Superfície IPC: `list_actions(workspaceId?)`, `approve_action(workspaceId, id)`, `create_action(input)`, `create_steps(workspaceId, parentId, steps)`, `continue_run(runId)` (ciclo com o próximo modelo da lista `haiku → sonnet → opus → fable`) e `start_run`/`preview_prompt` com `action` (e `continueFrom` opcional em `start_run`, para partir da branch de uma run concluída; `start_run` recebe também `reviewer` (`claude` ou `codex`), `reviewerModel` (vazio = automático) e `reviewWhen` (`auto`, `always`, `never`) e `instruction` opcional: texto de ajuste, até 4.000 caracteres, anexado ao fim do prompt para iterar numa run encadeada sem criar tarefa). `delete_task(workspaceId, taskId)` exclui a tarefa, o histórico e as runs dela (recusa se houver run em andamento; as branches ficam no git) e `review_run(runId, reviewer, reviewerModel)` revisa sob demanda uma run concluída com branch, sobre o diff `base..branch`, mesmo sem sinal de risco; o veredito e o ciclo entram na run sem mudar o estado dela nem o card. Não há permissão nova em `capabilities/main.json`, porque são comandos do próprio app.

Alterações: `run_diff(runId)` devolve os arquivos de `base..branch` (status, contagens, patch com limites) para a tela de leitura, sem escrever nada.

Automação: `get_automation(workspaceId, taskId?)`, `save_automation(workspaceId, modes, guardrails)`, `apply_automation_preset(workspaceId, preset)`, `set_automation_paused(workspaceId, paused)`, `save_task_automation(taskId, overrides)` e `list_automation_events(workspaceId, taskId?, limit)`. Modos: `manual`, `assisted`, `auto` por etapa (`start`, `review`, `qa`, `preview`, `publish`, `merge`); o núcleo recusa `auto` no merge. A configuração é local e pessoal, e o repositório só aperta as travas.

Ambiente de teste: `get_profile(workspaceId)`, `save_profile(workspaceId, run, port?)` e `approve_profile(workspaceId)` tratam `.harnessy/workspace.toml`; `start_preview(runId)`, `stop_preview(runId)`, `open_preview(runId)`, `preview_snapshot(runId)` e `list_previews()` controlam o processo (`run::preview`), com os eventos `preview-state` e `preview-log`. O comando só roda após confirmação do texto exato do arquivo e `open_preview` só abre `http://localhost:N` anunciado pelo ambiente.

Entrega: `publish_run(runId)` envia a branch e abre o PR em draft (idempotente), `refresh_delivery(runId)` relê checks, conflitos e merge feito por fora, `merge_delivery(runId)` faz squash do PR seguro e leva a tarefa a Concluído, e `list_deliveries(workspaceId)` lista os PRs. `git push` e `gh` rodam em `run::publish`, com o login do usuário, depois de verificar o diff (ADR 0001); o agente continua sem acesso a eles.

### Instruções do repositório

O prefixo do prompt diz "Siga o AGENTS.md", mas não copia o arquivo. Se o `claude` carrega o `CLAUDE.md` do worktree com os argumentos do Harnessy (`--setting-sources user`) é respondido pelo teste ignorado `instructions_are_loaded_by_the_cli` (`cargo test instructions_are_loaded_by_the_cli -- --ignored --nocapture`, consome um turno do `haiku`). Resultado: ainda não executado; registre aqui quando rodar.

### Medição

A suíte de [`experiments/economico`](../experiments/economico/README.md) compara cada modelo cru e com harness, aprovando por testes ocultos. `cargo test economico` (sem `--ignored`) só confere as soluções de referência das tarefas; a medição real consome tokens.

## Versionamento

### Fonte de verdade

`package.json.version` fornece a versão do produto. `src-tauri/tauri.conf.json` referencia `../package.json`; Vite importa o mesmo valor para a interface. `bun run check` valida o vínculo e o formato básico da versão.

`src-tauri/Cargo.toml` fornece a versão do núcleo Rust. Front e back são versionados de forma independente: cada modificação avalia os dois lados e aplica o bump do que foi afetado. O procedimento obrigatório está em `.claude/rules/versionamento.md`.

### Política SemVer

- **PATCH:** correção compatível.
- **MINOR:** funcionalidade compatível.
- **MAJOR, após 1.0:** mudança incompatível nos contratos públicos.
- **Durante 0.x:** mudança incompatível exige ao menos nova MINOR e migração explícita.
- Pré-releases usam nomes como `0.2.0-alpha.1` e `0.2.0-beta.1`.

Schema de dados/configuração e contratos de agentes/workflows terão versões próprias quando implementados. O rascunho atual usa schema `1`.

### Preparar uma versão local

1. Escolha a versão conforme a política e atualize `CHANGELOG.md`.
2. Atualize `package.json.version` e execute `bun install` para reconciliar o lockfile quando necessário. Não crie outro campo de versão na UI.
3. Execute os checks TypeScript/Rust e gere o bundle.
4. Confira a versão exibida no app e valide os fluxos afetados.

Esses comandos não criam uma release pública. Commit, tag, assinatura, notarização, publicação e updater ainda não têm pipeline configurado. Não existe repositório remoto oficial informado nesta etapa.

## Ícones

O desenho de origem está em `app-icon.svg`. Para regenerar os arquivos com a CLI local:

```bash
bun run tauri icon app-icon.svg
```

A CLI pode gerar também tamanhos para plataformas não utilizadas; o bundle desktop usa a lista de ícones de `tauri.conf.json`.

## PATH das ferramentas

Aplicativos abertos pelo Finder não herdam o `PATH` do shell, então o Harnessy lê uma vez o `PATH` do shell de login (`$SHELL -ilc`, com limite de 8 s) em segundo plano na abertura e o põe primeiro na busca de binários, nas verificações da run e no ambiente do agente. É assim que `node` é o do nvm e não o `/usr/local/bin`. `start_run` espera essa leitura terminar. Se o seu shell imprimir algo interativo ou demorar, o app cai no `PATH` comum.

## Bloqueios e custo de gestão

O agente não sabe que existe card, então gerir cards não custa token. O estado vem da run (tabela no [ADR 0003](adr/0003-estado-deduzido-sem-canal.md)) e o **bloqueio** vem de fatos que o Harnessy já recebe: a run falhou, terminou sem mudar nada ou o `claude` negou ações (`permission_denials`, que viram eventos `denied` no log). O texto fica em `runs.blocked`; o front mostra o selo "Bloqueada" quando a última run da tarefa terminou com bloqueio. Ao abrir o app, runs que sobraram viram `interrupted` e seus cards voltam a A fazer. Os sinais e as regras vivem em `flow::sync`.

## Smoke check antes de entregar

1. Abrir o desktop e confirmar a versão e os metadados locais.
2. Alternar telas, abas e painel de contexto por mouse e teclado.
3. Preparar tarefa, escrever, inserir sugestão e verificar que o texto foi preservado.
4. Aguardar salvamento e confirmar o rascunho após reabertura no mesmo contexto.
5. Abrir Sobre, fechar com Escape e conferir retorno do foco.
6. Abrir Conexões, confirmar o estado real de `gh`, `codex` e `claude` e usar “Verificar novamente”; conferir que o envio continua indisponível.
7. Em Tarefas, escolher o workspace, criar um card (`n`), movê-lo pelo botão, por `]`/`[` e arrastando, desfazer pelo aviso e conferir o histórico. Abrir o card, conferir o prompt e iniciar uma run pequena; conferir que o card vai para Em andamento e depois Revisão, que uma run que termina sem mudar nada ou com ações negadas ganha o selo "Bloqueada" e que "Marcar como resolvido" registra no histórico; acompanhar o log, cancelar uma segunda run e confirmar que a branch `harnessy/run-<id>-<slug>` existe no repositório. Fechar o app no meio de uma run e confirmar `Interrompida` ao reabrir.
8. Fechar janela, reabrir pela bandeja e encerrar explicitamente.

IPC e lifecycle precisam de verificação no Tauri real. Navegador/WebKit ajuda a validar a interface, mas não comprova a integração desktop. Para requisitos e próximos gates, consulte [SPEC.md](../SPEC.md).
