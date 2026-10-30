# Changelog

O produto segue SemVer. Durante 0.x, incompatibilidades exigem uma versão minor e migração explícita.

## [0.1.0] — Em desenvolvimento

- Harness de desenvolvimento: orientações por subtree, Biome, gates frontend/nativo, CI com permissões mínimas e auditoria local harness-score fixada.

- Fundação Tauri 2, Rust, React e TypeScript.
- Design tokens, navegação de workspace, board vazio, chat com rascunho local e conexões indisponíveis explicitamente.
- IPC somente de metadados; nenhuma ferramenta de shell ou credencial exposta à WebView.
- Ciclo de janela em segundo plano e menu de bandeja para reabrir/encerrar.
- Versão do produto centralizada em package.json.
- Refinamento UX: textos auxiliares mais legíveis, foco no compositor, sugestões não destrutivas e status real de salvamento do rascunho.
- Estado de digitação isolado do shell; adaptações para janela estreita e prévia web.
- README e guias de instalação, uso, diagnóstico e desenvolvimento com o estado real da fundação.
- Sessão de rascunho preservada entre telas, recuperação explícita de registro inválido e retry de gravação.
- Regressões de rascunho com Vitest; Bun para instalação/scripts e Vite para frontend, com `bun.lock`.
- Separação das telas Board/Conexões/Contexto e erros de lifecycle Rust observáveis.
- Workspaces e tarefas locais persistidos em SQLite, com controle de revisão para edições concorrentes.
- Conexões reais com GitHub, Codex e Claude Code via estado das CLIs oficiais (`gh`, `codex`, `claude`): detecção de instalação, sessão e conta, sem leitura ou armazenamento de credenciais; autenticação continua pelo terminal.
- Runtime fixo por papel: Claude Code refina e implementa, Codex revisa e testa (SPEC 1.2, `src/lib/roles.ts`).
