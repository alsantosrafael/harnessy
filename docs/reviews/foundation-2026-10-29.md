# Revisão da fundação — 2026-10-29

[← README](../../README.md) · [Fundação](../foundation.md) · [SPEC](../../SPEC.md)

## Escopo e método

Snapshot atual da fundação Tauri/Rust/React, sem comparação de commits: a pasta não possui repositório Git. Skills `code-review` e `codebase-design`, com duas avaliações independentes: padrões/arquitetura e aderência à SPEC. Confirmação posterior nos arquivos atuais e checks locais.

Não foram considerados defeitos os recursos explicitamente futuros: GitHub, Codex, SQLite, agentes, deploy e sandbox. Não foram alterados arquivos de implementação nesta revisão.

## Parecer

### Atualização após as correções

S1, S2, S3 e P1 estão corrigidos. Reavaliação independente confirmou os três caminhos de rascunho, sem novo bug concreto identificado. A sessão agora pertence à aplicação, o chat assina snapshots, leituras incompatíveis bloqueiam escrita e substituição exige ação explícita. Seis regressões Vitest passaram e uma sessão WebKit confirmou falha de gravação com navegação, retry, registro inválido, substituição explícita, whitespace e reload.

Board, Conexões e Contexto foram extraídos do shell. Erros de mostrar/ocultar/focar janela agora são registrados. Frontend migrou para Bun/Vite/Vitest com lockfile único `bun.lock`.

P2 foi parcialmente avançado: cinco inicializações do binário de produção processaram `app_info` real via IPC. P3 recebeu baseline parcial de início/RSS, registrado em [foundation.md](../foundation.md). Ainda faltam smoke de bandeja/Dock/fechamento e medições representativas de carga/latência de interação; não foram aprovados por inferência.

**Parecer atualizado:** caminhos de perda de rascunho resolvidos; pronto para continuar configuração/workspace, mantendo os gates desktop/carga visíveis e sem habilitar agentes antes de isolamento. O texto abaixo preserva os achados da revisão original, não descreve bugs ainda abertos.

**Aprovação condicional para continuar.** A escolha tecnológica e a separação entre apresentação e núcleo são adequadas. Antes de ampliar persistência e workspaces, corrigir os caminhos de perda de rascunho. Antes de declarar concluída a validação da fundação, fechar os gates de lifecycle desktop e desempenho.

## Standards — avaliação independente

### S1 · Média · Rascunho não salvo pode ser descartado na navegação

- `src/App.tsx:58–63`: o ramo de Conexões desmonta a área de workspace e o chat.
- `src/components/ChatPanel.tsx:15–24`: cleanup cancela debounce e tenta persistir, sem propagar erro.
- Se a gravação falha, o texto em memória desaparece ao mudar de tela; ao retornar, a leitura recupera somente o registro anterior. Isso contradiz a orientação de manter a janela aberta para preservar o texto e a preservação de rascunho da SPEC §5.
- Correção mínima: preservar a sessão de rascunho acima da navegação ou manter o chat montado; erros não devem destruir a única cópia em memória.
- Evidência: inspeção do lifecycle React; cenário de falha de armazenamento ainda não executado em browser nesta revisão.

### S2 · Média · Sugestões podem substituir conteúdo composto apenas por espaços

- `src/components/ChatPanel.tsx:31–34`: a condição `draft.trim()` trata espaços/quebras de linha existentes como ausência de texto.
- Diverge do contrato de inserção não destrutiva registrado no código e em `docs/foundation.md`.
- Correção mínima: decidir por comprimento zero, preservando literalmente o conteúdo existente.
- Evidência: inspeção estática. O impacto é restrito a conteúdo de whitespace, sem perda demonstrada de texto funcional.

### S3 · Média · Erro de leitura indistinguível de rascunho ausente

- `src/lib/drafts.ts:3–8`: leitura falha, JSON inválido e schema incompatível retornam `''`.
- `src/components/ChatPanel.tsx:9–18`: montagem grava esse valor mesmo sem edição humana.
- Um registro que poderia ser recuperado é sobrescrito ao abrir a interface.
- Correção mínima: resultado de leitura explícito (`missing`, `loaded`, `unreadable`/`incompatible`), preservando registro original e persistindo somente alterações autorizadas. A interface deve comunicar recuperação/descarte quando necessário.
- Julgamento de robustez apoiado pelos requisitos de preservação e compatibilidade.
- Evidência: reprodução executada com armazenamento em memória usando as funções reais: `{invalid-json` foi convertido em `{"schemaVersion":1,"text":""}` após leitura e salvamento equivalente ao efeito de montagem. Não foram alterados rascunhos reais.

## Spec — avaliação independente

### P1 · Média · Recuperação de rascunho incompatível é destrutiva

Mesmo caminho de S3, identificado independentemente: `src/lib/drafts.ts:5–8` e `src/components/ChatPanel.tsx:9–18`. Contraria preservação, recuperação e compatibilidade de schemas (SPEC §§5 e 15). Preservar o registro até decisão explícita de recuperação/descarte.

### P2 · Média · Gate desktop ainda não comprovado

`src/lib/desktop.ts:9–11`, `src-tauri/src/lib.rs:29–63` e evidências de `docs/foundation.md`: geração do bundle e processo aberto foram confirmados, mas não há registro suficiente de resposta IPC na janela real, fechar/reabrir por bandeja/Dock e encerramento explícito.

SPEC §§12 e 20 exige validação desktop real. Executar e registrar esse smoke check, incluindo versão, runtime e resultado. É lacuna de evidência, não defeito de lifecycle já reproduzido.

### P3 · Média · Gate de performance ainda aberto

SPEC §5 define metas, hardware/cenário e fixtures; `docs/foundation.md` e SPEC §19 mantêm as medições pendentes. Screenshots e tamanho do bundle não demonstram fluidez, memória ou estabilidade sob carga.

Registrar baseline de produção: abertura, resposta às ações, ociosidade, memória e streaming/carga representativa. A fixture volumosa será validada quando houver o caminho de dados correspondente; não declarar a meta aprovada apenas com layout vazio.

## Avaliação arquitetural consolidada

### Decisões adequadas

- React para apresentação e Rust para operações privilegiadas.
- Interface IPC pequena (`app_info`), sem shell ou filesystem genéricos expostos.
- Estado de digitação localizado no chat.
- Ausência de hierarquia de providers ou abstrações especulativas antes da prova Codex.
- Versionamento do produto centralizado e lockfiles dos dois ecossistemas.
- Capacidades futuras claramente identificadas na UI/documentação.

### Ajustes para manter a base evolutiva

- **Persistência do rascunho precisa de módulo mais profundo:** leitura, compatibilidade, gravação e falhas devem formar um contrato coerente; hoje parte dessa política está espalhada entre funções e efeitos React.
- **`App.tsx` precisa permanecer shell:** antes de adicionar board real e autenticação, extrair telas em módulos focados e formatar JSX/CSS para revisão legível. Não é preciso criar um módulo para cada detalhe estático.
- **Erros de lifecycle precisam ser observáveis ao evoluir:** chamadas `show`, `hide` e `set_focus` atualmente ignoram resultado; não expandir esse padrão para supervisor/execuções.
- **Isolamento continua gate futuro:** capabilities da WebView não validam sandbox de processos. Não habilitar executor Codex com base nesta revisão do shell.
- **Git ainda não foi inicializado:** antes de trabalho multiagente, PRs e releases, estabelecer histórico e ponto de comparação. Esta revisão não inicializa nem publica repositório.

Esses ajustes são recomendações arquiteturais; não aumentam a contagem dos relatórios independentes.

## Verificações executadas nesta revisão

- `npm run check` — passou.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` — passou.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — passou.
- `npm run build` — passou.
- Reprodução isolada da sobrescrita após falha de parsing — confirmou o caminho S3/P1.

Não foram reexecutados build desktop, avaliação visual ou benchmarks; resultados históricos não foram promovidos a verificações novas.

## Ordem recomendada

1. Corrigir contrato e lifecycle do rascunho (S1–S3/P1), com verificação de JSON/schema inválido e falha de gravação durante navegação.
2. Organizar o shell e padronizar formatação antes de acrescentar comportamento às telas.
3. Executar smoke de IPC/lifecycle desktop e registrar baseline de desempenho.
4. Avançar para workspace/configuração/SQLite em fatia vertical, mantendo a prova Codex/isolamento como gate separado.

**Resumo por eixo:** Standards: 3 achados, risco principal de perda de rascunho; Spec: 3 achados, risco principal de recuperação destrutiva e gates sem evidência. Há sobreposição S3/P1; as contagens não representam seis defeitos distintos.
