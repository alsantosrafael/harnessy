# Harness de desenvolvimento

[← README](../README.md) · [Desenvolvimento](development.md)

Este harness orienta Codex/OpenCode e verifica mudanças neste repositório. A orquestração de agentes no aplicativo continua sendo escopo futuro de `SPEC.md`. Os comandos abaixo não conectam integrações nem executam agentes pelo app.

## Uso diário

1. Confira `git status --short` e leia `AGENTS.md`; orientações específicas ficam em `src/AGENTS.md` e `src-tauri/AGENTS.md`.
2. Instale com `bun install --frozen-lockfile`, Bun 1.4.3 e Node 22+.
3. Faça a menor mudança necessária e execute o gate correspondente:

| Comando | Uso/custo |
| --- | --- |
| `bun run lint` | Biome: análise estática frontend, testes e scripts |
| `bun run format:check` | Formatação dos mesmos arquivos, sem escrever |
| `bun run format` | Aplicar formatação |
| `bun run check:quick` | Lint, formato, TypeScript, versão e Vitest |
| `bun run verify` | Gate rápido + build frontend de produção |
| `bun run verify:rust` | Rust fmt + clippy com lock e warnings como erros; primeira compilação é mais cara |
| `bun run harness:score` | Diagnóstico local de readiness/contexto |
| `bun run harness:report` | JSON em diretório temporário externo; imprime o caminho e preserva o exit code |

Use `bun run test:watch` no ciclo de rascunhos. Mudanças de IPC/lifecycle exigem ainda o smoke desktop descrito em `development.md`. Preserve as skills existentes em `.agents/skills`; não é necessário duplicar procedimentos em configurações de cada ferramenta.

## Decisões

- Biome 2.4.8 fornece lint e formato com uma configuração e binário local. CSS, Rust, docs e skills vendorizadas não são reformatados por esse gate. Rust usa rustfmt/clippy. A formatação inicial de TS/TSX/testes/scripts é mecânica.
- As regras recomendadas estão ativas, com `useButtonType` e `noNonNullAssertion` desativadas para adoção sem refatoração geral do frontend existente. O autofocus do modal tem exceção local justificada; o agrupamento de sugestões mantém layout com ARIA explícita. As demais regras de acessibilidade continuam ativas.
- A interface do harness são scripts do manifest: orientações encaminham para eles, sem novo framework de workflows. Não há uma nova seam de produto ou decisão de domínio que justifique ADR/glossário.
- Nenhum hook foi instalado: `core.hooksPath` estava ausente, não foi modificado. Pre-commit permanece decisão opt-in do mantenedor; não há `prepare` com efeitos de configuração. Os gates são explícitos e repetidos no CI.
- Não há guardrail executável de Codex/OpenCode configurado. Instruções e gitignore não bloqueiam leitura de segredos nem substituem sandbox. Use apenas dados de desenvolvimento no checkout e permissões nativas da ferramenta; não coloque credenciais em prompts/relatórios.

## CI

`.github/workflows/ci.yml` executa frontend em Linux com Node 22, Bun 1.4.3, lock frozen, lint/formato/tipos/testes/build e diagnóstico `harness:score --min-level 3`, com scanner fixado. O job macOS roda fmt/clippy apenas se o diff afetar `src-tauri`, manifest/lock ou o próprio workflow; base ausente causa execução conservadora. Node 22 também é explícito no job macOS. Os SHAs das três Actions foram confirmados com `git ls-remote` nos tags v4/v4/v2. Permissão: `contents: read`; execuções obsoletas são canceladas por ref. Não há secrets, publicação ou deploy. Rust stable acompanha atualizações; dependências Cargo permanecem travadas.

O workflow só poderá ser validado no GitHub após publicação autorizada. Configuração de branch protection não faz parte deste arquivo. CI nativo não substitui teste de bandeja/janela no desktop.

## Auditoria Paladini

Fonte consultada: [README oficial](https://github.com/paladini/harness-score) e tipos/código distribuídos de **harness-score 1.8.3**, versão confirmada no npm e fixada no manifest/`bun.lock`. Scanner local, sem chamadas LLM/rede; instalação e consulta de documentação usam rede, não enviam o código do projeto.

`.harness-score.json` define `user: false`, `system: false`, `extraRoots: []`, gate `maturity`, sem presets ou checks excluídos. Os relatórios indicam `maturity: [repo]`, `effective: [repo]`. O scanner inclui arquivos ignorados/untracked relevantes, pula diretórios gerados/dependências conhecidos, limita leitura de corpos a 512 KiB e não segue symlinks para fora da raiz. Diretórios pessoais não entram no scan. Gitignore não é filtro de privacidade desse scanner.

Relatórios brutos ficam **fora da árvore escaneada**, evitando evidência que auto-pontue. `harness:report` grava `harnessy-harness-reports/current.json` no `os.tmpdir()` (substituído em cada execução), criando diretório com modo 0700 e arquivo com 0600. Para guardar histórico, copie o resultado para armazenamento externo antes de repetir. O JSON pode conter evidências de arquivos: revise antes de compartilhar. O baseline desta sessão está em `/var/folders/w8/85s8_5ln4_38ccv4hzp3b2xh0000gn/T/opencode/harnessy-baseline.json`; é temporário, não um artefato durável do repositório. Use `bun run harness:score --diff /caminho/externo/baseline.json` para comparar snapshots compatíveis.

### Evidência registrada — 2026-10-29

Baseline capturado após instalar ferramentas fixadas e definir o escopo, antes das melhorias de contexto/CI. Portanto não representa uma árvore sem dependências de sensores: o scanner já identificava 20/20 em sensores, mesmo antes de os comandos de lint serem implementados e verificados.

| Dimensão | Antes | Depois |
| --- | ---: | ---: |
| Contexto | 6/20 | 17/20 |
| Skills | 7/17 | 7/17 |
| Hooks | 0/14 | 0/14 |
| Sensores | 20/20 | 20/20 |
| CI | 0/14 | 11/14 |
| Higiene | 18/20 | 18/20 |
| **Total** | **51/105 (49%), L0** | **73/105 (70%), L3 Sensing** |

Ambos: ferramenta 1.8.3, mesmo modelo/denominador, sem alteração de presets; verdicts maturity/effective `complete`, sem motivos de incompletude. HYG-08 não aplicável (sem MCP), por isso o denominador efetivo é 105, não os 108 anunciados no guia. Newly passing: CTX-03/04/05/06 e CI-01/02/03.

Limitações genuínas: CTX-02 reprova o guia curto por contar 17 linhas não vazias; SKL-04 aponta descrição curta da skill `pr`; não há comandos/subagentes customizados nem hooks/pre-commit. O scanner detecta Codex e Antigravity; detecção não comprova ferramentas realmente utilizadas. Bun/Cargo e comandos compostos podem ser reconhecidos parcialmente; a nota não substitui executar os scripts. Não adicionamos arquivos fictícios para L4.

HYG-05 aponta ausência de licença: **decisão pendente do dono**, sem licença fabricada. Scan não avalia qualidade dos testes, validade das instruções, isolamento de processos, branch protection ou segurança completa.

### Checks locais

- `bun install --frozen-lockfile`: passou com Bun 1.4.3 / Node 22.21.1.
- `bun run verify`: lint/formato (18 arquivos), TypeScript, vínculo versão 0.1.0, **6 testes Vitest**, build Vite: passou.
- `bun run verify:rust`: fmt e clippy `--locked -- -D warnings`: passou no macOS.
- Scan inicial/final e comparação: passaram, completos, escopo repo. Nenhum smoke desktop novo foi necessário: IPC/lifecycle não foram alterados.

## Diagnóstico

- Formato falhou: execute `bun run format`, revise o diff e repita o gate.
- Vitest falhou no worker: confira `node --version` (22+) e use `bun run test`, não `bun test`.
- Instalação frozen falhou: confira Bun 1.4.3; reconcilie manifest/lock com `bun install` somente quando a alteração de dependências for intencional.
- Rust falhou: confira Xcode Command Line Tools e rustfmt/clippy na toolchain; não silencie warnings globalmente.
- Scan sai com 2: procure verdicts/motivos de incompletude (permissões, symlinks externos, limite de arquivos). Não trate score incompleto como baseline autoritativo. Exit 1 é falha de gate quando um mínimo é solicitado; 0 é scan completo aprovado.
- Comparação mudou de modelo: compare versão, máximo, presets e escopo antes de atribuir variação ao código. Atualizações do scanner exigem novo baseline consciente.
