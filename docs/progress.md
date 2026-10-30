# Progresso

[← README](../README.md) · [Changelog](../CHANGELOG.md) · [Especificação](../SPEC.md)

Registro cronológico do que foi implementado, com o estado verificado em cada etapa. O `CHANGELOG.md` resume por versão; este arquivo explica o contexto e o que ficou pendente.

## 2026-10-30 — Harness econômico (spec 1.5, passos 1 a 6)

### Decisão

O núcleo fica fixo em duas capacidades — integrar ferramentas e mitigar alucinação — e cada opção do usuário vira uma Ação declarada em TOML ([spec](specs/harness-economico.md)). Nada assume um modelo: o harness se adapta ao escolhido e a tese é medida como ganho por modelo. A implementação foi feita por subagentes `haiku` (passos 1 a 4, revisados) e concluída pela sessão principal; o próprio fluxo serviu de piloto manual do harness.

### Entregue

- **Runtime e ledger (Back 0.5.0):** trait `Runtime` com adaptador Claude Code; `run_cycles` e totais por run, com `NULL` quando o uso não é informado.
- **Falha estruturada (0.6.0):** parsers de Vitest/Jest, `cargo test`, `cargo check`/`clippy`, `tsc`, pytest e `go test`; prompt de correção por falha, até 8 KB.
- **Ações (0.6.0, 0.10.0):** embutidas, do usuário e do repositório (confirmação por sha256), formulário **Nova Ação**, prefixo fixo do núcleo.
- **Contraprova e typecheck por padrão (0.7.0, 0.11.0):** testes novos precisam falhar sem a mudança; `exigir_prova` pede um teste num ciclo extra.
- **Continuidade e economia (0.8.0):** `--resume` nas correções com recuo, ciclo com o próximo modelo (`continue_run`), comandos negados lembrados, arquivos prováveis, esforço e teto de turnos por Ação, `--exclude-dynamic-system-prompt-sections`.
- **Revisor e passos (0.9.0):** revisor um modelo acima, só com sinal de risco, com achados filtrados contra o diff; "Dividir em passos" com tarefas encadeadas que continuam da branch anterior.
- **Medição:** `experiments/economico` com 4 tarefas reais, testes ocultos e soluções de referência conferidas por teste; matriz modelo × (cru, harness, revisor).

### Verificado

- `cargo test`: 126 passando, 3 ignorados (experimentos e teste de instruções, que chamam o `claude` real). `bun run verify:rust` e `bun run check:quick` (35 testes Vitest) verdes.
- Não verificado: smoke desktop (IPC mudou em vários passos), nenhuma run real com o `claude` depois destas mudanças, medição de `experiments/economico` (consome tokens) e o teste de carregamento do `CLAUDE.md`.

### Pendente

- Smoke desktop completo, incluindo revisor, "Dividir em passos", "Tentar com <modelo>" e confirmação de Ação do repositório.
- Rodar a medição e registrar `experiments/economico/RESULTADOS.md`; completar as tarefas reais até 10 (faltam refactor, critério ambíguo, baseline quebrado e tentação de enfraquecer teste).
- Pré-requisito da issue #2 (perfil `.harnessy/workspace.toml`, baseline, guarda do diff) segue aberto; a detecção de checks ainda é a de `verify::checks_for`.

## 2026-10-30 — Tarefas locais com estado, histórico e bloqueios

### Decisão

O Harnessy existe para digitar o mínimo: sem etapa de conexão com GitHub Project, as tarefas ficam só no SQLite local e o quadro as mostra em quatro raias (A fazer, Em andamento, Revisão, Concluído). O usuário move os cards, o Harnessy os move pelo resultado da run e cada movimento fica no histórico. Gerir cards não custa token: o agente não recebe instrução nem ferramenta ([ADR 0003](adr/0003-estado-deduzido-sem-canal.md)).

### Entregue

- `task`: estado por tarefa, `task_events` (histórico gravado na mesma transação), `move_task`, `task_history` e evento `task-changed`. Mover não altera `revision`.
- `flow::sync`: regra run → card, bloqueio deduzido de falha, run sem mudanças e ações negadas pelo `claude` (`runs.blocked`), recuperação de runs interrompidas na abertura e `resolve_block`. Um canal do agente chegou a existir (ADR 0002) e foi removido por custar token.
- Quadro único em `BoardPanel`/`Kanban`: mover por botão, teclado ou arrastar, com resposta imediata, desfazer e destaque do card movido; detalhe da tarefa com histórico.
- Removidos: conexão com GitHub Project, fonte GitHub, painel de preparação do ambiente e criação de Issues. Esquema editado no lugar (exceção da regra 13 enquanto não há dados a preservar).

### Verificado

- `bun run verify:rust`, `cargo test` (35 testes) e `bun run check:quick` verdes; Falta o smoke desktop completo, incluindo uma run com ação negada para ver o selo de bloqueio.

### Pendente

- Revisão pelo Codex (hoje o canal está pronto, o fluxo não); verificação do diff, push e PR (M2).

## 2026-10-29 — M1: run do agente em worktree

### Decisão

Fluxo Tarefa → Ação → Run → PR, com SQLite como fonte das tarefas e do estado operacional e execução do agente sem sandbox, restrita a workspaces confiáveis (ver [ADR 0001](adr/0001-execucao-sem-isolamento.md) e [GLOSSARY.md](../GLOSSARY.md)). O tester automatizado ficou fora de escopo: o usuário testa a branch manualmente.

### Entregue

- Migração 2: `workspaces.isolation`, tabelas `runs` e `run_events` (log limitado a 1 MB por run); runs ativas viram `interrupted` na abertura.
- `run` e `flow`: worktree em ramo `harnessy/run-<id>-<slug>`, `bun install --frozen-lockfile`, `claude -p` (Sonnet, 25 turnos, 15 min) com settings allow/deny, ambiente fixo e settings/MCP do projeto ignorados, cancelamento por grupo de processos, commit sem hooks e remoção do worktree em sucesso.
- `process.rs` virou infraestrutura compartilhada (cwd, timeout e limite configuráveis, leitura sem bloqueio de pipe).
- Frontend: prévia do prompt exato, botão "Implementar com Claude Code", runs com log em tempo real.
- Spike com `claude -p` real validou os guardrails (ADR).

### Verificado

- `bun run verify:rust` limpo; `cargo test` com 32 testes; `bun run verify` verde. Smoke desktop do item 7 ainda não executado.

### Pendente

- M2: verificação do diff (segredos, caminhos protegidos, tamanho), checks reexecutados, push e PR via `gh`.
- M3: Ações como arquivos em `.harnessy/actions/` e caixa de ferramentas. M4: revisor (Codex).

## 2026-10-29 — Runtime por papel: Claude Code implementa, Codex revisa

### Decisão

Cada papel de agente tem runtime fixo: **Refinador e Implementador usam Claude Code; Revisor e Tester usam Codex.** Substitui a decisão anterior da SPEC ("todos os papéis usam Codex"). Motivo: separar runtimes reforça a independência da revisão, que já não recebe a conversa privada do implementador.

### Entregue

- SPEC 1.2: objetivo, escopo e tabela de papéis atualizados com a coluna Runtime.
- `src/lib/roles.ts`: fonte única do mapeamento papel→runtime, com teste em `tests/roles.test.ts` garantindo que implementação e revisão nunca compartilham runtime.
- Painel de contexto lista os quatro papéis com o runtime de cada um; a tela de Conexões mostra quais papéis cada CLI atende.

### Pendente

- Execução real dos papéis (adaptadores Claude Code e Codex). A atribuição é configuração, não comportamento, até lá.
- Troca de runtime pela interface fica para o futuro, conforme SPEC.

## 2026-10-29 — Reorganização do núcleo Rust

### Entregue

- Crate dividido por domínio (`app`, `workspace`, `task`, `connections`) e infraestrutura (`db`, `error`); `lib.rs` só monta o `Builder`. Comandos IPC viraram adaptadores de uma linha em `*/commands.rs`; regras e SQL vivem em `mod.rs` sobre `&Connection`.
- `AppError` com `thiserror`, serializado como string: contrato com o frontend inalterado, sem vazamento de erros de biblioteca.
- Migrações incrementais em `db::MIGRATIONS`; `TempDatabase` para testes.
- Regras em `.claude/rules/rust-architecture.md` (escopo `src-tauri/**`) e documentação em `docs/architecture-rust.md`. `src-tauri/AGENTS.md` ficou mínimo e aponta para ambos.

### Verificado

- `bun run verify:rust` limpo; `cargo test` com 7 testes (parsers, migrações, workspaces, tarefas).

## 2026-10-29 — Trabalho local e conexões reais

### Entregue

- **Workspaces e tarefas locais (SQLite):** módulo `src-tauri/src/local_work.rs` (hoje `db.rs`, `workspace/` e `task/`) com `rusqlite` (bundled), schema `user_version=1`, controle de revisão otimista nas edições e uma aba de tarefas (hoje `BoardPanel.tsx`). Banco em `app_data_dir/local-work.sqlite3`.
- **Conexões GitHub, Codex e Claude Code:** módulo `src-tauri/src/connections.rs` (hoje `connections/`) com o comando `check_connections`. Procura as CLIs `gh`, `codex` e `claude` no PATH e em diretórios usuais, executa só o comando de status de cada uma (entrada fechada, tempo limite de 20 s, saída limitada a 16 KB) e devolve estado, conta e caminho. A tela **Conexões** exibe os estados Conectado, Sessão expirada, Indisponível e Não verificado, com o comando de login ou instalação quando necessário. Credenciais nunca chegam à WebView.
- **Codex CLI** instalado globalmente com `bun install -g @openai/codex` (0.166.0) em `~/.bun/bin`.
- Documentação atualizada: README, guia do usuário, troubleshooting e development.

### Verificado

- `bun run verify`: lint, formatação, TypeScript, 6 testes Vitest e build de produção.
- `bun run verify:rust`: `cargo fmt --check` e `clippy -D warnings`; 4 testes unitários do parser de status.
- Teste ao vivo na máquina de desenvolvimento: GitHub conectado, Claude Code conectado (claude.ai), Codex instalado sem sessão.

### Decisões

- Autenticação continua no terminal do usuário, conforme a SPEC (reaproveitar a autenticação suportada). O app orienta e verifica; não abre fluxos OAuth próprios nem guarda tokens.
- Claude Code entrou na tela de Conexões como runtime alternativo, antecipando a extensão prevista na SPEC. A execução de agentes por qualquer runtime continua não implementada.

### Pendente

- Smoke check visual no desktop (`bun run start`) da tela de Conexões após estas mudanças.
- Autenticar o Codex (`codex login`) na máquina de desenvolvimento.
- Próximas etapas da SPEC: seleção de repositórios e execução de agentes com worktrees.
