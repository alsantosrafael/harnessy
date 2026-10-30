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
| `connections::process` | Execução controlada de binários | Sistema operacional |

Dependências só apontam para baixo. Dois domínios nunca se importam: se compartilham algo, isso vira módulo de infraestrutura.

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
    mod.rs                 # Task, TaskInput, list, save com revisão otimista
    commands.rs            # list_tasks, save_task
  connections/
    mod.rs                 # Provider, State, ConnectionStatus, check_connections
    probe.rs               # Tabela estática: binário, args de status, dicas
    process.rs             # search_dirs, find_binary, run com timeout e limite
    interpret.rs           # Saída da CLI → Verdict; funções puras testadas
```

## Decisões

**Erro único serializado como string.** `AppError` é um enum `thiserror` cuja mensagem é o texto exibido ao usuário. `impl Serialize` entrega só essa string, então o frontend continua tratando erros como `string` e nenhum detalhe de `rusqlite` ou do sistema vaza para a WebView. Novas situações viram variantes; mensagens de validação usam `Validation(&'static str)`.

**Domínio sobre `&Connection`, não sobre `State`.** Funções como `task::save` e `workspace::create` recebem a conexão já obtida. Isso permite testá-las com um banco temporário sem subir o Tauri e mantém `commands.rs` com uma linha por comando.

**Migrações incrementais por `user_version`.** `db::MIGRATIONS` é uma lista ordenada; o índice +1 é a versão gravada. Abrir um banco com versão maior que a lista falha com `SchemaTooNew`, protegendo dados criados por um Harnessy mais novo. Alterar esquema é sempre adicionar ao final.

**Conexões divididas em dados, efeito e lógica.** `probe.rs` é a única fonte de quais comandos rodam; `process.rs` é o único lugar que cria processos; `interpret.rs` não tem efeitos colaterais e é testado com texto fixo. A verificação roda em threads paralelas e devolve `Unknown` em vez de propagar pânico.

**Ciclo de vida fora do `Builder`.** `app::setup`, `tray::on_window_event` e `tray::on_run_event` são funções nomeadas, não closures inline. `lib.rs` fica legível e cada comportamento de janela tem um lugar para ser testado manualmente no smoke desktop.

## Como estender

| Preciso de... | Faço... |
| --- | --- |
| Novo comando em domínio existente | Função em `mod.rs` sobre `&Connection`; adaptador em `commands.rs`; registrar em `lib.rs`; tipar em `src/lib/desktop.ts` |
| Novo domínio | `src/<nome>/mod.rs` + `commands.rs`; declarar em `lib.rs`; testes com `TempDatabase` |
| Nova coluna ou tabela | Nova string ao final de `MIGRATIONS`; ajustar `from_row` e `COLUMNS` |
| Nova mensagem de erro | Variante em `AppError` ou `Validation("...")` |
| Nova CLI verificada | Entrada em `PROBES`, braço em `interpret.rs`, teste com saída real capturada |

## Verificação

```sh
bun run verify:rust                      # fmt --check + clippy -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Mudanças em IPC, janela ou bandeja exigem o smoke desktop descrito em [development.md](development.md).
