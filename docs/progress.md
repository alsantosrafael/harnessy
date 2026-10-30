# Progresso

[← README](../README.md) · [Changelog](../CHANGELOG.md) · [Especificação](../SPEC.md)

Registro cronológico do que foi implementado, com o estado verificado em cada etapa. O `CHANGELOG.md` resume por versão; este arquivo explica o contexto e o que ficou pendente.

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

- **Workspaces e tarefas locais (SQLite):** módulo `src-tauri/src/local_work.rs` (hoje `db.rs`, `workspace/` e `task/`) com `rusqlite` (bundled), schema `user_version=1`, controle de revisão otimista nas edições e aba **Tarefas locais** em `LocalWorkPanel.tsx`. Banco em `app_data_dir/local-work.sqlite3`.
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
- `reload` nos efeitos do `LocalWorkPanel` é um gatilho intencional; anotado com `biome-ignore`.

### Pendente

- Smoke check visual no desktop (`bun run start`) da tela de Conexões após estas mudanças.
- Autenticar o Codex (`codex login`) na máquina de desenvolvimento.
- Próximas etapas da SPEC: seleção de repositórios e GitHub Projects, board com cards reais, execução de agentes com worktrees.
