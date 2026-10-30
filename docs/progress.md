# Progresso

[← README](../README.md) · [Changelog](../CHANGELOG.md) · [Especificação](../SPEC.md)

Registro cronológico do que foi implementado, com o estado verificado em cada etapa. O `CHANGELOG.md` resume por versão; este arquivo explica o contexto e o que ficou pendente.

## 2026-10-29 — Trabalho local e conexões reais

### Entregue

- **Workspaces e tarefas locais (SQLite):** módulo `src-tauri/src/local_work.rs` com `rusqlite` (bundled), schema `user_version=1`, controle de revisão otimista nas edições e aba **Tarefas locais** em `LocalWorkPanel.tsx`. Banco em `app_data_dir/local-work.sqlite3`.
- **Conexões GitHub, Codex e Claude Code:** módulo `src-tauri/src/connections.rs` com o comando `check_connections`. Procura as CLIs `gh`, `codex` e `claude` no PATH e em diretórios usuais, executa só o comando de status de cada uma (entrada fechada, tempo limite de 20 s, saída limitada a 16 KB) e devolve estado, conta e caminho. A tela **Conexões** exibe os estados Conectado, Sessão expirada, Indisponível e Não verificado, com o comando de login ou instalação quando necessário. Credenciais nunca chegam à WebView.
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
