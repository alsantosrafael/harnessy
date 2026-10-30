# Arquitetura do núcleo Rust

[← README](../README.md) · [Desenvolvimento](development.md) · Regras: [`.claude/rules/rust-architecture.md`](../.claude/rules/rust-architecture.md)

O crate em `src-tauri` é o único lugar com acesso privilegiado: SQLite, processos externos, janela e bandeja. A WebView fala com ele exclusivamente por comandos IPC tipados. Este documento descreve como o crate é organizado e por quê; as regras operacionais para quem edita o código estão no arquivo de rules acima, que é carregado automaticamente em qualquer tarefa que toque `src-tauri/`.

## Camadas

```text
WebView (React) ──invoke──▶ commands.rs ──▶ domínio (mod.rs) ──▶ db / process
                            adaptador        regras + modelos     efeitos
```

| Camada | Responsabilidade | Conhece |
| --- | --- | --- |
| `lib.rs` | Registrar módulos e montar o `Builder` | Todos os módulos, só pelos nomes públicos |
| `*/commands.rs` | Extrair `State`, delegar, devolver `Result` | Domínio e `db::Database` |
| `*/mod.rs` (domínio) | Validação, modelos, SQL, decisões | `rusqlite::Connection`, `error` |
| `db`, `error` | Conexão, migrações, erro único | Nada do domínio |
| `process` | Execução controlada de binários de curta duração | Sistema operacional |
| `flow`, `app` | Composição: orquestram fluxos que cruzam domínios | Vários domínios |

Dependências só apontam para baixo. Dois domínios nunca se importam: se compartilham algo, isso vira módulo de infraestrutura; só `flow` e `app` compõem vários domínios.

## Mapa de módulos

```text
src-tauri/src/
  main.rs                  # Entrada do executável; chama harnessy_lib::run
  lib.rs                   # Builder, invoke_handler, setup e eventos
  error.rs                 # AppError (thiserror) + Result; serializa como string
  db.rs                    # Database(Mutex<Connection>), MIGRATIONS, TempDatabase de teste
  app/
    mod.rs                 # app_info, setup (banco + bandeja), show_main_window
    tray.rs                # Menu da bandeja, ocultar ao fechar, reabrir pelo Dock
  workspace/
    mod.rs                 # Workspace, list, create, validação de nome e pasta
    commands.rs            # list_workspaces, create_workspace
  task/
    mod.rs                 # Task, Status, transition/annotate/history, save com revisão otimista
    commands.rs            # list_tasks, save_task, move_task, task_history; evento task-changed
  process.rs               # search_dirs, find_binary, run/run_with com timeout, cwd e limite
  connections/
    mod.rs                 # Provider, State, ConnectionStatus, check_connections
    probe.rs               # Tabela estática: binário, args de status, dicas
    interpret.rs           # Saída da CLI → Verdict; funções puras testadas
  run/
    mod.rs                 # Run, RunEvent, estados, uma run ativa por workspace, log limitado
    guard.rs               # Argumentos, settings (allow/deny), ambiente fixo, lista de modelos e esforço
    runtime.rs             # Contrato neutro com o runtime (AgentSpec, AgentEvent, CycleUsage, Runtime)
    claude.rs              # Adaptador do Claude Code: único módulo que conhece a CLI
    prompt.rs              # Início fixo do prompt (regras do núcleo); a Ação entra só como pedido final
    stream.rs              # Linhas stream-json → eventos e resultado final
    redact.rs              # Remove padrões de token e trunca antes de gravar
    worktree.rs            # git worktree, bun install, commit sem hooks, slug de branch
    supervisor.rs          # Único spawn de longa duração: stream, limite de tempo, cancelar
    commands.rs            # list_runs, list_run_events, cancel_run
  flow/
    mod.rs                 # Pipeline worktree → agente → checks → correção → commit → contraprova → revisor, via trait Sink
    sync.rs                # blocker, reconcile (run → card), recover (abertura), resolve (bloqueio); sobre &Connection
    commands.rs            # preview_prompt, start_run, continue_run, list/approve/create_action (+ Sink com banco e eventos Tauri)
    actions.rs             # Catálogo com validação de modelo, esforço e turnos; montagem do prompt
    read.rs                # Ação de leitura: agente na pasta do workspace, sem worktree; saída `passos`
    verify.rs              # Checks do perfil (Tipos antes de Testes), execução e prompt de correção
    failure.rs             # Saída de check → Failure (arquivo, linha, teste, esperado/obtido) por runner
    proof.rs               # Contraprova: testes novos falham sem a mudança?
    review.rs              # Revisor: diff, filtro de achados contra linhas alteradas, Failure de revisão
    likely.rs              # Arquivos prováveis (termos da tarefa × git ls-files/grep)
    experiment.rs          # Experimento do loop (teste ignorado)
    economico.rs           # Medição por modelo com testes ocultos (teste ignorado) + conferência das tarefas
  action/
    mod.rs                 # Formato TOML da Ação, variáveis, catálogo (repositório > usuário > embutida), aprovação por hash, Ação nova
    embutidas/             # implementar, explicar, revisar e dividir-em-passos, compilados com include_str!
```

## Decisões

**Erro único serializado como string.** `AppError` é um enum `thiserror` cuja mensagem é o texto exibido ao usuário. `impl Serialize` entrega só essa string, então o frontend continua tratando erros como `string` e nenhum detalhe de `rusqlite` ou do sistema vaza para a WebView. Novas situações viram variantes; mensagens de validação usam `Validation(&'static str)`.

**Domínio sobre `&Connection`, não sobre `State`.** Funções como `task::save` e `workspace::create` recebem a conexão já obtida. Isso permite testá-las com um banco temporário sem subir o Tauri e mantém `commands.rs` com uma linha por comando.

**Migrações incrementais por `user_version`.** `db::MIGRATIONS` é uma lista ordenada; o índice +1 é a versão gravada. Abrir um banco com versão maior que a lista falha com `SchemaTooNew`, protegendo dados criados por um Harnessy mais novo. Alterar esquema é sempre adicionar ao final.

**Conexões divididas em dados, efeito e lógica.** `probe.rs` é a única fonte de quais comandos rodam; `process.rs` cria os processos de curta duração e `run::supervisor` o único de longa duração; `interpret.rs` não tem efeitos colaterais e é testado com texto fixo. A verificação roda em threads paralelas e devolve `Unknown` em vez de propagar pânico.

**Ciclo de vida fora do `Builder`.** `app::setup`, `tray::on_window_event` e `tray::on_run_event` são funções nomeadas, não closures inline. `lib.rs` fica legível e cada comportamento de janela tem um lugar para ser testado manualmente no smoke desktop.

## Como estender

| Preciso de... | Faço... |
| --- | --- |
| Novo comando em domínio existente | Função em `mod.rs` sobre `&Connection`; adaptador em `commands.rs`; registrar em `lib.rs`; tipar em `src/lib/desktop.ts` |
| Novo domínio | `src/<nome>/mod.rs` + `commands.rs`; declarar em `lib.rs`; testes com `TempDatabase` |
| Nova coluna ou tabela | Nova string ao final de `MIGRATIONS` (em 0.x pré-release pode-se editar no lugar, ver regra 13); ajustar `from_row` e `COLUMNS` |
| Nova mensagem de erro | Variante em `AppError` ou `Validation("...")` |
| Nova CLI verificada | Entrada em `PROBES`, braço em `interpret.rs`, teste com saída real capturada |

## Verificação

```sh
bun run verify:rust                      # fmt --check + clippy -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Mudanças em IPC, janela ou bandeja exigem o smoke desktop descrito em [development.md](development.md).
