# Fundação desktop — 0.1.0

[← README](../README.md) · [Guia de uso](user-guide.md) · [Desenvolvimento](development.md)

## Decisões implementadas

- Tauri 2 usa a WebView do sistema; Electron foi removido das dependências e do código.
- Rust controla lifecycle e fornece apenas o comando IPC `app_info`. Não existe executor de shell, leitor genérico de arquivos ou acesso a segredos pela UI.
- React representa somente navegação, rascunho e apresentação. Não assume que conexões ou agentes estão ativos.
- `package.json` fornece a versão distribuída ao Tauri e ao build React; o crate não é publicado independentemente.
- Rascunho local é conveniência da interface, não persistência de domínio. SQLite virá em fatia própria. Não armazenar credenciais no rascunho.
- A sessão de rascunho permanece no aplicativo independentemente da tela, preserva erros em memória e exige ação explícita para substituir registros incompatíveis.
- Frontend usa Bun 1.4.3, Vite e Vitest. Dependências em `bun.lock`; `bun run test` executa Vitest (não o runner nativo do Bun).
- Janela oculta no fechamento; menu de bandeja reabre/encerra. Não há daemon instalado ou agente em execução.

## Verificações executadas

Em macOS Apple Silicon, Node 22 e Rust 1.95:

- TypeScript estrito e verificação de versão centralizada.
- Build frontend Vite de produção.
- `cargo check`, `cargo fmt --check` e `cargo clippy -- -D warnings`.
- Bundle Tauri macOS gerado, aproximadamente 8,57 MiB com ícones desktop configurados.
- Aplicativo aberto pelo LaunchServices; processo nativo permanece ativo após inicialização.

Isso não comprova revisão visual, acessibilidade assistiva, IPC em automação desktop, comportamento do menu ou metas de desempenho. Memória, tempo de abertura e fixture de carga ainda não foram medidos. O tamanho do bundle não é memória residente.

### Revisão UX posterior

Interface revisada visualmente por screenshots WebKit e exercitada via Playwright CLI:

- Preparar tarefa move foco ao compositor.
- Sugestão adiciona texto sem substituir rascunho existente.
- Rascunho permanece após reload.
- Escape fecha o diálogo e devolve foco ao acionador.
- Setas alternam seleção das abas.
- Sem overflow horizontal nas larguras 1360, 1000 e 375 px; 375 é prévia web, não requisito de janela macOS.
- Movimento reduzido aplica duração de transição `0s`.
- Console do navegador sem erros/warnings na sessão verificada.

Textos auxiliares foram ampliados, estados de salvamento separados e estado de digitação extraído para o módulo de chat. Esta validação é da interface web em WebKit, não de acessibilidade assistiva ou IPC desktop. As metas de performance continuam pendentes de medição representativa.

### Estabilização após revisão arquitetural

- Seis testes Vitest verificam leitura sem escrita, preservação de registro inválido/incompatível/grande, falha de leitura, falha de escrita com remount/retry, whitespace/limite e debounce.
- WebKit com falha de armazenamento injetada confirmou que ir para Conexões e voltar não perde texto; retry salva quando o armazenamento volta.
- Registro corrompido permaneceu intacto ao abrir/editar até clicar explicitamente em substituir. Rascunho salvo foi preservado após reload.
- Shell passa a compor `BoardPanel`, `ChatPanel`, `ConnectionsPage` e `ContextPanel`; sessão e persistência estão em `lib/drafts.ts`.
- TypeScript incluindo testes/configuração Vitest, formatação Rust, Clippy e build desktop passaram. Bundle após esta mudança: aproximadamente 8,47 MiB.

### Baseline nativo inicial — 2026-10-29

Binário de produção lançado diretamente cinco vezes em macOS 27.0.1, arm64, MacBookAir10,1. Tempo medido de criação do processo até o núcleo registrar o recebimento de `app_info`, emitido pelo renderer real:

| Tentativa | Processo → recebimento IPC |
| --- | --- |
| 1 | 1.975,5 ms |
| 2 | 453,5 ms |
| 3 | 425,9 ms |
| 4 | 416,3 ms |
| 5 | 436,0 ms |

Mediana: 436,0 ms; máximo da amostra: 1.975,5 ms. Não é p95 validado, tempo até paint interativo nem teste formal de cold start. As medições foram feitas antes da troca do executor frontend para Bun, sobre o mesmo comportamento Rust/React corrigido; o bundle Bun posterior foi compilado, não remensurado.

Um segundo após cada recebimento IPC, RSS do processo principal ficou entre 96.544 e 104.912 KiB. Esses valores não somam processos auxiliares WebKit e não são orçamento total do aplicativo. `%CPU` instantâneo reportado por `ps` variou de 0,3 a 1,5 e não comprova consumo sustentado em ociosidade. Processos de medição foram encerrados por sinal, portanto não validam os menus de encerramento.

O IPC nativo foi observado, mas smoke de bandeja/Dock/fechamento e carga representativa continuam pendentes. Não extrapolar esta amostra para metas gerais de performance.

## Skills aplicadas

- `codebase-design`: seam pequeno em `lib/desktop.ts`, sem camada genérica de provider antecipada.
- `design-system`: tokens primitivos → semânticos → componentes, compartilhados entre telas.
- `ui-ux-pro-max`: foco, teclado nas abas, retorno de foco em diálogo, estados vazios e ausência de funcionalidades fictícias.
- Vercel React Best Practices: efeitos com cleanup, persistência com debounce e estado localizado no shell inicial. Virtualização será introduzida com dados e perfis reais.

## Próxima fatia recomendada

Configuração aprovada e workspace local persistido: definir o contrato Rust, schema SQLite e migração inicial; confirmar o seam de testes antes da implementação. Em paralelo ao desenho dessa fatia, medir o shell com carga representativa. Não habilitar Codex antes da prova de isolamento e autenticação descrita na SPEC.
