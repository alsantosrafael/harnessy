# Spec — Harness plug-and-play

[← SPEC](../../SPEC.md) · [Progresso](../progress.md) · [ADR 0001](../adr/0001-execucao-sem-isolamento.md)

Versão do documento: 1.0 · 2026-10-30 · Status: referência de longo prazo. O escopo do primeiro entregável está na issue [#2](https://github.com/alsantosrafael/harnessy/issues/2), que substitui as fatias F1–F3 e F8 desta versão.

## 1. Objetivo

Transformar o fluxo Tarefa → Run → PR em um harness que:

1. **Funciona em qualquer repositório** com poucos cliques, sem que o usuário edite configuração à mão.
2. **Tira o melhor resultado de modelos mais simples**, porque o núcleo monta o contexto, verifica o resultado e conduz as correções, em vez de depender da disciplina do modelo.
3. **Pode ser usado por quem não domina git, terminal ou agentes**, com cada estado explicado e uma próxima ação sempre visível.

## 2. Problema: o que o código faz hoje

| Ponto | Hoje | Consequência |
| --- | --- | --- |
| Verificação | `flow::pipeline` roda `claude -p` uma vez; saída 0 vira `completed`. Quem roda os checks é o próprio agente, porque o prompt pede. | Um modelo fraco declara sucesso sem verificar, ou "corrige" enfraquecendo o teste, e o Harnessy aceita. |
| Stack | `guard::ALLOW` só permite `bun` e `cargo`; `worktree::install_deps` só conhece Bun. | Em um repositório Python, Go ou npm, o agente não consegue instalar dependências nem rodar testes. |
| Configuração | `.harnessy/` não existe; o setup de §12 da SPEC não foi construído. | Nada adapta a Run ao projeto. |
| Contexto | `prompt::implement` envia título, descrição e "siga o AGENTS.md". | Sem `AGENTS.md`, o modelo não recebe orientação; com critérios vagos, ele adivinha. |
| Tamanho da tarefa | Uma tarefa = uma Run de até 25 turnos. | Modelos menores perdem o rumo em tarefas longas. |
| Modelo | `guard::MODEL = "sonnet"` fixo. | Não dá para usar nem medir modelos menores. |
| Fim do fluxo | Sem push, PR ou revisor (M2 e M4 pendentes). | O usuário precisa terminar no terminal. |
| Entrada do usuário | Login e escopo do `gh` resolvidos no terminal, sem guia; README desatualizado em relação a `docs/progress.md`. | Um usuário leigo trava antes da primeira Run. |

## 3. Princípios

1. **O harness não confia no modelo.** Sucesso é decidido por checks executados pelo núcleo e pelo diff verificado, nunca pela mensagem final do agente.
2. **Determinístico antes de modelo.** Tudo que pode ser calculado (comandos, arquivos relevantes, falhas, limites) é calculado em Rust e entregue pronto ao agente.
3. **Passos pequenos e verificáveis.** Quanto mais simples o modelo, menor o passo e mais curto o ciclo de feedback.
4. **Detectar, propor, confirmar.** O Harnessy detecta o projeto e propõe; o usuário confirma em linguagem simples. Nada vira autoridade sem confirmação local (SPEC §15).
5. **Toda tela responde três perguntas:** o que está acontecendo, por que, e o que fazer agora.
6. **Medir antes de afirmar.** "Funciona com modelo X" só vale com resultado de avaliação registrado.

## 4. Público

**Usuário-alvo:** sabe descrever o que quer construir e tem um repositório no GitHub, mas não domina git, terminal ou configuração de agentes.

**Usuário avançado:** continua com acesso aos detalhes técnicos (log, prompt, comandos, diff), sempre por divulgação progressiva e nunca como primeira camada.

## 5. Escopo

| Fatia | Entrega | Depende de |
| --- | --- | --- |
| F1 | Perfil do workspace detectado e confirmado | — |
| F2 | Loop de verificação e guarda do diff | F1 |
| F3 | Pacote de contexto e critérios de aceite obrigatórios | F1 |
| F4 | Decomposição em passos com checkpoint | F2, F3 |
| F5 | Publicação: push e PR pelo Rust | F2 |
| F6 | Revisor Codex no ciclo | F5 |
| F7 | Modelo por papel e avaliações | F2, F3 |
| F8 | Onboarding e experiência da Run | F1, F2 (cresce com as demais) |
| F9 | Documentação sincronizada com o estado real | contínua |

Ordem de entrega: **F1 → F2 → F3 → F8 (onboarding + Run) → F5 → F6 → F4 → F7**. F2 e F3 são o núcleo do ganho com modelos simples; F8 entra cedo porque sem ela o usuário-alvo não chega à primeira Run.

## 6. F1 — Perfil do workspace

### Deve

- Detectar a stack **só lendo arquivos** da raiz e de subpastas de primeiro nível:

  | Sinal | Instalação proposta | Checks propostos |
  | --- | --- | --- |
  | `bun.lock` / `bun.lockb` | `bun install --frozen-lockfile` | scripts `lint`, `typecheck`/`check`, `test` do `package.json` |
  | `package-lock.json` | `npm ci` | idem, via `npm run` |
  | `pnpm-lock.yaml` | `pnpm install --frozen-lockfile` | idem, via `pnpm run` |
  | `yarn.lock` | `yarn install --immutable` | idem, via `yarn` |
  | `Cargo.toml` + `Cargo.lock` | — (o build resolve) | `cargo check`, `cargo test`, `cargo clippy` quando instalado |
  | `pyproject.toml` com `uv.lock` | `uv sync --frozen` | `uv run pytest`, `ruff check` quando configurado |
  | `pyproject.toml` com `poetry.lock` | `poetry install --no-root` | `poetry run pytest` |
  | `go.mod` | `go mod download` | `go vet ./...`, `go test ./...` |
  | `AGENTS.md` / `CLAUDE.md` com comandos de verificação | — | comandos citados viram sugestão, nunca aceitos sem confirmação |

- Gravar o perfil confirmado em `.harnessy/workspace.toml`, com `schema = 1`:

  ```toml
  schema = 1

  [install]
  command = ["bun", "install", "--frozen-lockfile"]
  timeout_s = 300

  [[check]]
  name = "Testes"
  command = ["bun", "run", "test"]
  timeout_s = 300
  required = true

  [protect]
  paths = [".github/workflows/**", "**/*.lock", "migrations/**"]
  ```

- Comandos são **listas de argumentos**, nunca strings de shell. Binário resolvido pelo `process::find_binary`.
- Gerar a partir do perfil: a lista `allow` do `--settings` (cada check vira `Bash(<binário> <args> *)`), o comando de instalação e a lista de caminhos protegidos (deny de edição).
- Registrar a aprovação do perfil **localmente** (SQLite: hash do arquivo aprovado por workspace). Perfil alterado no repositório volta a exigir confirmação antes da próxima Run.
- Oferecer **"Testar verificações"**: roda instalação e checks no worktree de teste e mostra o resultado antes de qualquer Run (é também o baseline inicial).
- Permitir perfil sem checks, com aviso explícito: "Sem verificações, o Harnessy não consegue confirmar que o agente acertou."

### Não deve

- Executar qualquer comando durante a detecção.
- Aceitar comando de shell livre (`sh -c`, `bash -c`, pipes, `&&`) no perfil.
- Deixar o arquivo do repositório ampliar permissões sem confirmação local (SPEC §15).
- Pedir que o usuário escreva TOML: a edição acontece na interface, e o arquivo é o resultado.
- Suportar monorepo com vários perfis nesta fatia (um perfil por workspace; subpasta como raiz é permitida).

### Critérios de aceite

- Em repositórios-fixture Bun, npm, Cargo, uv e Go, a detecção propõe instalação e checks corretos sem executar nada.
- Uma Run em repositório Python consegue rodar `pytest` e é negada ao tentar `curl`.
- Editar `.harnessy/workspace.toml` fora do app faz a próxima Run pedir nova confirmação.

## 7. F2 — Loop de verificação e guarda do diff

### Deve

- Executar no Rust, com o mesmo ambiente restrito do agente (`guard::env`):
  1. **Baseline:** checks obrigatórios no worktree recém-criado, antes do agente. Falhas preexistentes são registradas e não contam contra a Run.
  2. **Agente** implementa.
  3. **Verificação:** o núcleo roda os checks obrigatórios.
  4. **Correção:** se algum check que passava no baseline falhou, o núcleo reinvoca o agente com um prompt de correção (§8) contendo apenas o nome do check, o comando e o trecho final da saída (até 8 KB por check, sem códigos de cor).
  5. Repetir até os checks passarem ou atingir **3 ciclos** (SPEC §8). Infraestrutura (timeout de instalação, binário ausente) não conta como ciclo.
- **Guarda do diff**, antes de aceitar o resultado:
  - caminhos protegidos do perfil e os já negados no `guard` (`.git`, `.claude`, `.harnessy`, `.env*`, lockfiles);
  - limite de tamanho (padrão: 50 arquivos ou 2.000 linhas alteradas; acima disso a Run pede decisão humana);
  - padrões de segredo, usando os mesmos padrões de `run::redact`;
  - **sinais de teste enfraquecido:** arquivo de teste removido, `skip`/`only`/`xit`/`#[ignore]`/`@pytest.mark.skip` adicionados, asserções removidas sem substituição. Detectar não bloqueia sozinho: vira aviso destacado na revisão humana e no revisor (F6).
- Estados da Run (alinhados à SPEC §16): `preparing` → `checking` (baseline) → `implementing` → `checking` → (`implementing` em ciclo N) → `completed` | `awaiting_input` | `failed`. O ciclo atual é um campo da Run (`cycle`, `max_cycles`), não um estado novo.
- `completed` exige: checks obrigatórios verdes (ou com falha preexistente idêntica ao baseline) e guarda do diff sem bloqueio.
- Ao esgotar os ciclos: estado `awaiting_input`, worktree preservado, resumo do que falhou e ações "Tentar mais um ciclo", "Abrir a pasta" e "Descartar".
- Registrar cada check como evidência: comando, duração, código de saída, trecho da saída, SHA avaliado.

### Não deve

- Aceitar a mensagem final do agente como prova de sucesso.
- Remover o worktree de uma Run que não terminou em `completed` ou `no_changes`.
- Reenviar ao agente a saída completa dos checks ou o log inteiro da Run.
- Fazer retry infinito ou silencioso; todo ciclo é visível.
- Corrigir falhas de baseline sem pedido: falha preexistente é informada, não atribuída à Run.

### Critérios de aceite

- Fixture com bug proposital: o agente introduz um erro de tipo, o check falha, o ciclo 2 corrige e a Run termina `completed` com dois registros de check.
- Fixture em que o agente apaga um teste: a Run termina com o aviso de teste enfraquecido visível.
- Repositório com teste já quebrado no `main`: a Run não falha por isso e mostra "1 falha já existia antes".

## 8. F3 — Pacote de contexto e critérios de aceite

### Deve

- **Critérios de aceite obrigatórios** para iniciar uma Run de implementação. Fontes, em ordem: seção `## Critérios de aceitação` da Tarefa; lista de checkboxes da descrição; resultado do Refinador.
- Sem critérios: o botão principal do card passa a ser **"Refinar"**. O Refinador (Claude Code, somente leitura: `Read`, `Glob`, `Grep`) lê a tarefa e o código e devolve critérios verificáveis e no máximo 3 perguntas. O usuário edita e confirma. Os critérios confirmados ficam com a Tarefa local.
- O prompt de implementação é montado em Rust com seções fixas e orçamento total (padrão 24 KB):
  1. Tarefa e critérios numerados.
  2. Comandos de verificação do perfil, exatamente como o núcleo vai rodá-los.
  3. Mapa do repositório: árvore de até 2 níveis, limitada a 150 entradas, sem pastas ignoradas pelo git.
  4. Arquivos prováveis: até 10 caminhos encontrados por busca determinística (termos do título e dos critérios contra nomes de arquivo e `git grep -l`), apenas caminhos, sem conteúdo.
  5. Instruções do repositório: conteúdo de `AGENTS.md`/`CLAUDE.md` da raiz, truncado em 4 KB.
  6. Regras fixas (não fazer push, não editar caminhos protegidos, terminar com o bloco de saída).
- Prompt de correção (F2): critérios, checks que falharam e trechos da saída, mais o resumo do ciclo anterior. Nada do histórico de conversa.
- **Saída estruturada:** o agente termina com um bloco JSON (`{"criteria":[{"id":1,"status":"done|partial|not_done","note":"…"}],"notes":"…"}`). O núcleo valida o schema; bloco ausente ou inválido não falha a Run, mas aparece como "o agente não reportou critérios".
- Mostrar ao usuário o prompt exato antes da Run (já existe), agora organizado pelas seções acima.

### Não deve

- Enviar conteúdo de arquivos inteiros, histórico de conversa ou o log de outra Run.
- Ultrapassar o orçamento: seções são cortadas da última para a primeira, com marca `[truncado]`.
- Usar o Refinador como gerente que roda em toda etapa (SPEC §14).
- Tratar o JSON do agente como prova: o status dos critérios é a opinião do agente, exibida ao lado dos checks, que são a evidência.

### Critérios de aceite

- Tarefa sem critérios não oferece "Implementar"; oferece "Refinar" e explica por quê.
- O prompt de uma Run em repositório sem `AGENTS.md` ainda contém comandos de verificação, mapa e arquivos prováveis.
- Teste de unidade garante o orçamento de 24 KB com tarefa, mapa e instruções grandes.

## 9. F4 — Decomposição em passos

### Deve

- Quando a tarefa tiver mais de 4 critérios ou o Refinador marcá-la como grande, propor um plano de até 5 passos, cada um com critérios próprios.
- Executar cada passo como um ciclo de F2 no mesmo worktree, com commit de checkpoint ao final de cada passo verde.
- Permitir ao usuário aceitar, editar ou executar como passo único.

### Não deve

- Rodar passos em paralelo ou com escritores simultâneos nesta fatia.
- Avançar para o passo seguinte com checks vermelhos.

## 10. F5 — Publicação

### Deve

- Ao fim de uma Run `completed`: push da branch `harnessy/run-<id>-<slug>` e `gh pr create` pelo Rust (regra 28), com corpo contendo tarefa, critérios e status, checks com resultado, avisos da guarda do diff e identificação da Run.
- Registrar a intenção antes do efeito e reconciliar depois (procurar PR existente da branch antes de criar), para não duplicar após retry ou crash.
- Checks vermelhos com publicação pedida pelo usuário: PR em draft, identificado como tal.
- Pedir confirmação na primeira publicação de cada workspace, mostrando repositório, branch e conta.

### Não deve

- Fazer merge, force push ou publicar sem a Run ter passado pela guarda do diff.
- Dar ao agente acesso a `git push` ou `gh` (continua negado no `guard`).

## 11. F6 — Revisor Codex

### Deve

- Revisar exatamente o SHA candidato, recebendo tarefa, critérios, diff, checks e avisos da guarda; nunca a conversa do implementador.
- Devolver veredito estruturado (`approve | request_changes | inconclusive`, achados com severidade e caminho/linha) e publicá-lo como comentário de PR pelo Rust.
- `request_changes` volta ao implementador como novo ciclo, dentro do mesmo limite de 3.

### Não deve

- Rodar com permissão de escrita no worktree.
- Bloquear indefinidamente: `inconclusive` vai para decisão humana.

## 12. F7 — Modelo por papel e avaliações

### Deve

- Modelo por papel configurável na interface, gravado localmente (não no repositório), com padrão atual (`sonnet` para implementação).
- Suíte de avaliação local: repositório-fixture com 10 tarefas iniciais (bug simples, feature pequena, refactor, tarefa ambígua, teste quebrado no baseline, tentativa de enfraquecer teste), cada uma com grader determinístico (estado final de arquivos + checks).
- Comando `bun run eval -- --model <m> --trials 3` que registra por modelo: taxa de tarefas validadas, ciclos médios, duração p50/p95 e consumo quando observável (SPEC §14).
- Exibir na escolha de modelo o último resultado da suíte para aquele modelo, ou "não avaliado".

### Não deve

- Roteamento automático entre modelos ou providers.
- Afirmar que um modelo é recomendado sem resultado da suíte.
- Usar grader por modelo sem calibração humana (SPEC §18).

## 13. F8 — Experiência do usuário

Diretrizes de base: SPEC §5 (tokens, WCAG 2.2 AA, teclado, estados) e a skill `ui-ux-pro-max`. Esta seção define os fluxos.

### 13.1 Primeira execução: assistente "Começar"

Substitui a tela de Conexões como primeira experiência quando não há workspace configurado. Cinco passos com indicador **"Passo 2 de 5"**, botão **Voltar** sempre disponível e estado salvo entre aberturas do app.

| Passo | O que mostra | Verificação | Quando falha |
| --- | --- | --- | --- |
| 1. Ferramentas | `git`, `gh`, Claude Code; Codex marcado como opcional até F6 | `connections` atual | Comando de instalação exato com botão **Copiar** e link para a página oficial |
| 2. Contas | Sessão de cada CLI, conta exibida | Status da CLI | Comando de login com **Copiar**; o app reverifica ao voltar ao foco, sem polling |
| 3. Projeto | Seletor nativo de pasta | É repositório git? Tem remote GitHub? Tem commits? | Explica o que falta em uma frase e oferece a correção possível |
| 4. Verificações | Perfil detectado (F1) em linguagem simples: "Para instalar: `bun install`", "Para conferir: Testes, Lint" | **Testar verificações** roda o baseline | Mostra qual check falhou e permite seguir mesmo assim, com aviso |
| 5. Confiança e primeira tarefa | Explica em 3 linhas o que o agente pode e não pode fazer (ADR 0001), pede confirmação de que o projeto é confiável; oferece criar a tarefa de teste (setup existente) | Confirmação explícita | — |

Regras:
- Cada passo tem **uma ação principal** e diz por que ela é necessária.
- Passos já resolvidos aparecem como concluídos e podem ser pulados automaticamente, com o motivo visível ("GitHub já conectado como @conta").
- Não há tour sobreposto obrigatório; o assistente pode ser fechado e retomado pela barra lateral.
- Falhas trazem causa e ação. Nunca exibir stderr bruto na primeira camada; ele fica em **Detalhes técnicos**.

### 13.2 Board

- Cada card mostra título, critérios (contagem ou "sem critérios") e o estado da última Run com ícone e texto.
- Uma ação principal por card, decidida pelo estado: **Refinar** (sem critérios) → **Implementar** → **Ver progresso** (Run ativa) → **Revisar resultado** → **Ver PR**.
- Ação indisponível mostra o motivo ao lado ("Outra Run está ativa neste projeto"), não apenas botão desabilitado (SPEC §5).
- Estado vazio: cada raia diz o que acontece nela e a primeira oferece o campo **Nova tarefa**.

### 13.3 Acompanhar uma Run

- **Linha do tempo de etapas** fixa no topo: Preparar → Instalar → Conferir antes → Implementar → Conferir → Corrigir (ciclo 2 de 3) → Revisar → Publicar. Cada etapa tem ícone, texto e duração em algarismos tabulares; a atual fica destacada sem depender só de cor.
- Abaixo, **o que está acontecendo agora** em uma frase ("Rodando Testes — 1 de 3 verificações"), publicada em uma única região `role="status"` com `aria-atomic`, sem anunciar cada linha de log.
- **Verificações:** lista com aprovado/falhou/já falhava antes, expandindo a saída sob demanda.
- **Log do agente:** recolhido em **Detalhes técnicos**, virtualizado, sem rolagem forçada quando o usuário está lendo acima e com indicador de novidades (SPEC §5).
- **Cancelar:** confirmação que diz o efeito ("O agente para. As alterações ficam na pasta da Run para você revisar.").
- Notificação do macOS ao terminar quando a janela está oculta.

### 13.4 Resultado

- Cartão de resultado com: o que mudou (arquivos, linhas), critérios com status do agente ao lado da evidência dos checks, avisos da guarda do diff em destaque e ação principal (**Ver alterações**, depois **Publicar PR**).
- **Ver alterações:** lista de arquivos + diff unificado carregado por arquivo, sob demanda; diffs grandes começam recolhidos.
- Falha ou `awaiting_input`: blocos **O que aconteceu**, **Por que** e **O que você pode fazer**, com as ações de F2.

### 13.5 Linguagem

- Português direto, frases curtas, verbo de ação nos botões.
- Termos do [GLOSSARY](../../GLOSSARY.md) mantidos, com explicação curta na primeira aparição (ex.: Worktree — "cópia isolada do projeto onde o agente trabalha").
- Mensagens de erro vêm das variantes de `AppError` (regra 10), sempre com ação de recuperação no frontend.

### 13.6 Visual e acessibilidade

- Tokens existentes em `src/styles/tokens.css`; nenhuma cor literal nos componentes novos.
- Texto essencial em `--text-base` (13 px, padrão macOS) ou maior; `--text-xs` só para metadados.
- Estados sempre com ícone + texto, foco visível, alvo mínimo de 24×24 px, alternativa de teclado para arrastar (já existe "Mover para").
- Respeitar `prefers-reduced-motion`; animações só para transição de estado.
- Ícones vetoriais de uma única família; sem emoji como ícone.

### Não deve

- Tour obrigatório, modal para fluxo principal ou assistente que bloqueia o resto do app.
- Spinner sem dizer qual etapa está rodando.
- Jargão na primeira camada (stderr, exit code, SHA, prompt) — tudo isso existe, mas em **Detalhes técnicos**.
- Prometer o que não foi verificado ("Pronto!" antes dos checks).
- Polling contínuo de CLIs; reverificação só em foco da janela ou por botão.

### Critérios de aceite

- Usuário em um Mac com as CLIs instaladas mas sem login conclui o assistente usando o terminal apenas para colar os comandos de login fornecidos.
- Navegação completa do assistente, do board e da Run por teclado, com foco visível e status anunciado.
- Nenhum texto de erro na primeira camada contém saída bruta de CLI.

## 14. F9 — Documentação

### Deve

- Atualizar `README.md` (tabela "O que está disponível"), `docs/user-guide.md` e `docs/progress.md` na mesma mudança que entrega cada fatia.
- Registrar novos termos no `GLOSSARY.md` antes de usá-los no código.

### Não deve

- Documentar como disponível algo não verificado no app desktop real.

## 15. Fora do escopo desta spec

| Item | Motivo / quando |
| --- | --- |
| Sandbox de processo e contêiner | Permanece o ADR 0001. **Gate:** nenhuma distribuição a terceiros nem workspace não confiável antes de uma spec própria de isolamento. |
| Deploy, ambiente remoto e Playwright (SPEC §12) | Depende do provedor piloto; spec própria. |
| Multi-repositório e subagentes paralelos | Após F4 estável. |
| Importar de Jira/Linear/GitHub Issues | Hoje as tarefas são só locais; exigiria um adaptador novo. Sem demanda imediata. |
| Execução automática por mudança de card ou webhook | Fora do escopo do produto (SPEC §3). |
| Providers via API e roteamento automático de modelos | Após F7 produzir dados comparativos. |
| Windows e Linux | SPEC §3. |

## 16. Riscos

| Risco | Mitigação |
| --- | --- |
| Detecção de stack errada | Sempre confirmada pelo usuário; "Testar verificações" antes da primeira Run. |
| Checks lentos tornam o loop caro | Timeout por check no perfil; checks marcados `required = false` rodam só na verificação final. |
| Modelo entra em ciclo de correção inútil | Limite de 3 ciclos e parada em `awaiting_input` com evidência. |
| Heurística de teste enfraquecido com falso positivo | É aviso para revisão, não bloqueio. |
| Usuário leigo confia demais no resultado | Cartão de resultado separa evidência (checks) de opinião (relato do agente) e PR nunca é mergeado automaticamente. |
| Comando de check com efeito colateral (ex.: script que publica) | Perfil só com comandos confirmados, ambiente sem tokens, rede do agente negada para `curl`/`gh`; risco residual do ADR 0001 permanece. |

## 17. Definição de pronto da spec

A spec está cumprida quando, em repositórios-fixture Bun e Python e no próprio Harnessy:

1. Um usuário sem configuração prévia conclui o assistente e inicia a primeira Run sem editar arquivos.
2. Uma tarefa sem critérios passa pelo Refinador antes de implementar.
3. Uma Run com erro introduzido pelo agente se corrige pelo loop e só termina `completed` com checks verdes.
4. O resultado vira PR pelo Rust, sem duplicação após repetir a ação, com evidências no corpo.
5. O Codex revisa o SHA exato e um `request_changes` gera novo ciclo.
6. A suíte de avaliação roda com dois modelos e registra a comparação.
7. `bun run verify`, `bun run verify:rust`, `cargo test` e o smoke desktop passam; README e guia descrevem o estado real.
