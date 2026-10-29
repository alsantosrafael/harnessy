# Práticas de harness para um app local Tauri, Rust e React

**Data da pesquisa: 29 de outubro de 2026.** Fontes consultadas diretamente: artigos e documentação oficiais Anthropic/OpenAI, API pública do GitHub, README e manifests dos projetos identificados; documentação Tauri como complemento específico da stack. Este documento contém achados e recomendações, não uma alteração de requisitos do produto.

## Resumo executivo

- Harness é o sistema que envolve o modelo: contexto, ferramentas, execução, permissões, persistência e feedback. Avaliar o agente significa avaliar **modelo + harness**, e não apenas o prompt. [A3]
- Trabalho prolongado exige checkpoints verificáveis e recuperação entre sessões; apenas aumentar a janela ou compactar não resolve continuidade. [A1, A2]
- Para incorporar **o agente Codex existente** em uma interface desktop, app-server é a interface indicada para autenticação, histórico, aprovações e eventos. O SDK Codex automatiza esse agente; não é o SDK genérico da API de modelos. [O2, O3]
- Para um **loop próprio via API**, o app assume execução das funções, estado, controle de orçamento, recuperação e isolamento. Uma chamada de ferramenta é uma solicitação do modelo, não uma autorização para executar. [O6, O8]
- Medir separadamente qualidade real, prontidão estática do repositório, consumo observado e custo estimado. `harness-score` não substitui evals comportamentais. Há pelo menos dois projetos distintos com esse nome. [H1–H5]

## 1. Evidências: harness engineering e agentes long-running

### Anthropic

O artigo de 26/11/2025 relata falhas como tentar implementar tudo de uma vez, perder contexto com trabalho incompleto e declarar conclusão cedo demais. A solução experimental combina uma sessão inicializadora com sessões incrementais: lista estruturada de funcionalidades, script de inicialização, registro de progresso, histórico Git e teste básico do estado existente antes de avançar. O exemplo trabalha uma funcionalidade por sessão e só a marca como concluída depois de verificá-la. Inicializador e executor diferem principalmente pelo prompt inicial, não necessariamente por infraestrutura ou processos separados. [A1]

**Aplicação recomendada:** persistir objetivo, critérios de aceite, trabalho em curso, evidências dos checks, arquivos alterados e próxima ação. Separar `implementado` de `verificado`. Checkpoints não devem depender apenas de a última mensagem afirmar sucesso. Antes de retomar, confrontar checkpoint com o filesystem atual. Commits são uma técnica do experimento, não autorização para o produto fazer commits automaticamente. [A1; recomendação derivada]

### OpenAI

O relato de 11/02/2026 defende documentação navegável no repositório, um `AGENTS.md` curto como índice, regras arquiteturais verificadas mecanicamente e aplicação/logs/métricas acessíveis ao agente. A equipe tornou a aplicação inicializável por worktree e descreve execuções de mais de seis horas. Os números de produtividade e autonomia são observações de um produto interno, não garantias gerais ou um benchmark controlado. [O1]

**Aplicação recomendada:** fornecer comandos reproduzíveis para Rust e React, documentação pequena com links, contratos IPC tipados e logs por execução. Para tarefas concorrentes, usar workspace/worktree e dados de teste separados; validar a aplicação desktop real além da página React no navegador. Não transferir automaticamente a política de merges permissiva do relato interno para este produto. [O1; recomendação derivada]

## 2. Contexto e continuidade

Anthropic recomenda o menor conjunto de tokens de alto sinal suficiente para a tarefa, recuperação just-in-time por referências e ferramentas claras com pouca sobreposição. Para longos horizontes, distingue compactação, anotações persistentes e subagentes. Compactar agressivamente pode perder informação crítica; subagentes são uma alternativa contextual, não uma exigência universal. [A2]

OpenAI documenta histórico explícito, Conversations e encadeamento com `previous_response_id`. O identificador anterior não elimina cobrança: tokens anteriores da cadeia continuam sendo cobrados como entrada. Na reprodução manual, preservar os itens de saída exigidos pelo protocolo, inclusive itens de reasoning opacos e campos de fase quando aplicáveis; não reconstruir a conversa apenas com texto visível. Persistência remota de conversa também não equivale a armazenamento exclusivamente local. [O7]

**Recomendações derivadas para o app:**

1. Separar transcript de transporte, estado operacional e resumo de retomada. Resumos devem conter referências às evidências originais.
2. Carregar sob demanda trechos de código e resultados; limitar tamanho das saídas e oferecer paginação/caminho para o artefato completo.
3. Preservar decisões, restrições, verificações executadas, falhas abertas e próximo passo ao compactar; testar retomadas em tarefas reais.
4. Registrar versão do modelo, prompt, adaptador e política de contexto para comparar regressões.
5. Se houver delegação futura, incluir seus tokens, latência e falhas no custo e resultado da tarefa principal.

## 3. Integração local: agente externo Codex versus loop próprio

| Aspecto | Agente externo Codex | Loop próprio sobre API |
| --- | --- | --- |
| Quem conduz ferramentas e inferência | Runtime Codex, controlado pelo cliente | Orquestrador da aplicação |
| Interface | app-server; SDK Codex para automação | Responses/function calling ou API equivalente |
| Estado | Threads/turns/items e retomada Codex | Histórico e checkpoints mantidos pelo app, com opções de persistência da API |
| Permissões | Configuração/sandbox Codex + integração correta das aprovações | Política, executor e sandbox implementados pelo app |
| Principal responsabilidade do app | Ciclo de vida, UX, eventos, aprovações e observabilidade | Todo o loop, além da UX e observabilidade |
| Dependência operacional | Runtime Codex e sua versão/autenticação | Credenciais e contrato da API escolhida |

Base factual: [O2, O3, O4, O6, O7]. A divisão arquitetural abaixo é recomendação, não uma implementação já auditada neste repositório.

### Caminho recomendado para uma interface Codex rica

`React → comandos/eventos Tauri → supervisor Rust → codex app-server (stdio)`.

- O app-server usa mensagens JSON-RPC bidirecionais, sem o campo `jsonrpc` no fio. O transporte padrão é stdio com JSON delimitado por newline. Inicializar a conexão com `initialize` e `initialized`, criar/retomar thread e iniciar turn. Drenar continuamente notificações e tratar solicitações iniciadas pelo servidor. [O2]
- Modelar UI por **thread → turn → item**; deltas de mensagem não significam término. A documentação distingue `turn/completed` e `turn/interrupt`; interrupção não deve ser apresentada como rollback de arquivos. [O2; última implicação derivada]
- Manter IDs de correlação, buffers limitados e stderr separado do canal de protocolo. Persistir referências para retomada e tratar saída inesperada do filho. Não reenviar cegamente uma ação com efeitos após desconexão; primeiro reconciliar o estado. [Recomendação derivada de O2]
- Gerar schemas com a mesma versão do CLI usada na integração; a documentação fornece `generate-ts` e `generate-json-schema`. Fixar/testar a versão do runtime e negociar funcionalidades experimentais explicitamente. [O2]
- Preferir stdio para este caso local. A documentação consultada marca WebSocket como experimental/não suportado e alerta que listeners fora de loopback podem aceitar conexões sem autenticação por padrão durante rollout. Não expor esse controle à rede por conveniência. [O2]

### Onde o SDK se encaixa

O SDK TypeScript inicia, continua e retoma threads locais, requer Node.js 18+ e deve executar no lado servidor. Portanto, não deve ser importado como se fosse biblioteca de navegador dentro do React/WebView; precisaria de runtime auxiliar. A documentação atual também descreve SDK Python controlando app-server, com runtime CLI fixado nas distribuições publicadas. Para Rust/Tauri, o protocolo stdio evita exigir Node apenas para esse adaptador. Não foi confirmado um SDK Rust oficial independente nesta pesquisa. [O3; decisão de integração derivada]

### Se o produto construir seu próprio agente

No fluxo de function calling, a API devolve chamadas e argumentos; a aplicação executa código e devolve resultados associados ao `call_id`, repetindo até resposta final ou limite operacional. Schemas estritos melhoram formato, mas não substituem validação de caminhos, autorização ou controle de efeitos. [O6; implicação de segurança derivada]

Recomenda-se um executor Rust com ferramentas pequenas e tipadas, limites de turnos/tempo/tokens, cancelamento, tratamento de múltiplas chamadas e registro durável antes/depois de efeitos. Ferramentas mutáveis precisam de reconciliação/idempotência própria. Credenciais e execução permanecem fora do frontend. Isso é um produto de orquestração diferente de uma interface sobre o Codex; manter adaptadores separados evita misturar seus contratos de estado e cobrança.

## 4. Segurança e isolamento

Anthropic enfatiza **filesystem e rede em conjunto**, com enforcement do SO cobrindo subprocessos. O artigo cita Seatbelt/macOS e bubblewrap/Linux, e descreve credenciais Git fora do sandbox no caso cloud. As alegações de redução de prompts são resultados internos, não prova de segurança absoluta para outro app. [A4]

OpenAI distingue sandbox, que impõe fronteiras técnicas, de aprovação, que decide quando pedir autorização. Modos e opções variam por versão e plataforma. O modo de acesso irrestrito remove fronteiras importantes; aprovação automática não cria isolamento. A documentação sobre segurança também reconhece que prompt injection e vazamento não são eliminados por prompts ou guardrails. [O4, O8]

**Recomendações para Tauri/Rust/React:**

- Limitar capabilities da WebView e oferecer comandos específicos (`start_run`, `interrupt_turn`, `respond_approval`) em vez de um comando genérico para shell. A documentação Tauri ressalta que capabilities não protegem contra Rust inseguro ou checagem incorreta de escopo; comandos registrados pelo app merecem revisão explícita de exposição. [T1]
- Tratar arquivos do repo, páginas, resultados MCP e stdout como dados não confiáveis. Não promovê-los a instruções privilegiadas; renderizar conteúdo sem HTML executável e validar argumentos no backend. [O8; recomendação de UI derivada]
- Usar raízes de leitura/escrita explícitas, resolver symlinks e impedir escapes de caminho. `cwd` e worktree ajudam organização e concorrência, mas não são sandbox de segurança. [A4, O4; implicação derivada]
- Aplicar isolamento ao processo executor e seus descendentes; limitar também saídas de rede das ferramentas. Não confundir a rede necessária à inferência com acesso irrestrito à internet por comandos. [A4, O4]
- Guardar chaves no mecanismo seguro do SO/backend, retirar segredos dos logs e reduzir ambiente herdado pelos filhos. Não distribuir uma chave de fornecedor compartilhada embutida no bundle. [Recomendação derivada]
- Aprovações devem mostrar ação, diretório/recursos e escopo, estar vinculadas à solicitação correta e expirar quando a execução termina. Testar negação, cancelamento e solicitações pendentes após crash. [O2, O4; recomendação derivada]
- Para evals e tarefas concorrentes, separar diretórios, bancos locais, portas e artefatos. Isolamento experimental evita contaminação de resultados; isolamento de segurança exige enforcement adicional. [A3, O1, A4]

## 5. Evals: medir resultados do sistema

Anthropic distingue task, trial, grader, trace e outcome. O outcome é o estado final real, não a declaração do agente. Recomenda múltiplas tentativas, ambientes limpos, graders determinísticos quando possível e julgamento por modelo calibrado com humanos quando necessário. Sugere começar com 20–50 casos reais. `pass@k` mede obter ao menos um sucesso; `pass^k` mede consistência em todas as tentativas. [A3]

OpenAI recomenda objetivos específicos, datasets representativos, avaliação contínua e calibração humana; arquitetura multiagente deve ser justificada por evals. **Limitação temporal importante:** a página consultada informa que a plataforma Evals ficará read-only em 31/10/2026 e encerrará em 30/11/2026. As práticas permanecem úteis, mas não se recomenda acoplar uma nova implementação a essa plataforma. [O5]

**Suite inicial proposta para o app local:**

| Caso | Evidência de sucesso |
| --- | --- |
| Alterar projeto Rust/React pequeno | Checks independentes passam e diff atende ao pedido |
| Retomar após crash/compactação | Objetivo e restrições preservados; efeito já confirmado não é repetido |
| Cancelar durante ferramenta | Estado terminal correto, subprocessos tratados e efeitos remanescentes informados |
| Negar aprovação | Ação negada não ocorre; UI não trava nem aprova outra solicitação |
| Arquivo malicioso com prompt injection | Tentativa de escapar de escopo é bloqueada pelo executor |
| Paths com espaços, symlinks e traversal | Acesso permitido funciona; escape é recusado |
| Duas execuções simultâneas | Sem mistura de eventos, arquivos ou dados locais |
| Credencial ausente, rate limit e rede interrompida | Erro recuperável, estado reconciliável e retries limitados |
| Contexto longo e outputs grandes | Sem perda silenciosa de restrições ou crescimento ilimitado de buffers |

Executar cada trial em fixture limpa, com versão/configuração registradas; guardar resultado funcional, violações de escopo, intervenções humanas, tentativas, tempo total e consumo. Avaliar estado final por testes fora do controle do agente quando possível. Não exigir uma sequência exata de ferramentas quando várias soluções são válidas. Calibrar rubricas de qualidade com revisão humana e reportar tamanho da amostra/variabilidade. [A3, O5; suite proposta]

## 6. Medição de consumo e custos

**Separar três valores:** consumo observado, estimativa tarifária e cobrança reconciliada. A tabela OpenAI distingue entrada, entrada em cache, saída, faixas de contexto, modos de processamento e ferramentas cobradas separadamente. Definições de ferramentas também consomem tokens. O encadeamento por ID não torna contexto anterior gratuito. [O9, O6, O7]

Modelo de cálculo recomendado, com categorias mutuamente exclusivas:

```text
custo_estimado = Σ(categoria_de_tokens × tarifa_da_categoria / 1_000_000)
                + Σ(chamadas/armazenamento/execução cobrados por ferramenta)
```

Não somar tokens de cache duas vezes dentro do total de entrada, nem reasoning duas vezes quando já incluído na saída. Categorias e tarifas devem seguir o provedor, modelo, faixa e data. Incluir retries, resumos, delegações e graders no total atribuído à tarefa. [O9; recomendação de contabilização]

Registro mínimo proposto: `run_id`, `task_id`, provedor/adaptador, modelo efetivo, versão do runtime, thread/turn/request IDs disponíveis, timestamps, status, tokens por categoria, chamadas de ferramentas, latência, tabela tarifária datada e origem do valor (`observado`, `estimado`, `reconciliado`). Tratar ausência de dados como **desconhecido**, nunca zero. Em eventos cumulativos do agente externo, reconciliar deltas para evitar somar o mesmo consumo repetidamente.

Para Codex autenticado por plano, não inferir uma fatura em dólares simplesmente multiplicando tokens pela tarifa da API: registrar modalidade de autenticação/cobrança e rotular qualquer conversão como estimativa equivalente, quando válida. A disponibilidade de custos e campos deve ser confirmada no schema da versão usada. [O2, O3; recomendação e limite]

Anthropic oferece relatórios organizacionais de uso/custo e recomenda atualmente a Usage and Cost Analytics API para novas integrações; os endpoints Admin antigos continuam suportados. Credenciais e disponibilidade variam por conta/produto e a Admin API não está disponível para contas individuais. Portanto, reconciliação administrativa deve ser opcional; não exigir chave administrativa para o app funcionar. A primeira URL tentada para “usage-and-costs” retornou 404; a fonte válida está em [A5].

Métricas úteis: custo total por tentativa, custo por tarefa aprovada pelos graders (incluindo tentativas malsucedidas), p50/p95 de duração, taxa de recuperação, cache hit quando observável e intervenções humanas por tarefa. Limites de orçamento locais devem considerar chamadas já em voo; cancelamento não desfaz consumo já realizado. [A3, O9; recomendações derivadas]

## 7. `harness-score`: identidade encontrada e ambiguidade real

A tentativa de usar `gh api` falhou porque `gh` não está instalado. A pesquisa continuou diretamente pela API HTTPS pública do GitHub, sem instalação. A busca ampla retornou muitos resultados irrelevantes; a busca `harness-score in:name` retornou 59 resultados e foram lidos os primeiros cinco, incluindo **dois repositórios de nome exatamente igual**. Não se presume unicidade nem que um deles seja o projeto que o solicitante tinha em mente. [H1, H2]

### Candidato A: `paladini/harness-score`

- Repositório: <https://github.com/paladini/harness-score>. Autor indicado no manifest do CLI: **Fernando Paladini**; licença MIT. O manifest lido em `main` declara pacote npm `harness-score`, versão **1.8.3**, Node >=18 e API programática ESM. Isso não prova qual versão estava publicada no registry no momento. [H3, H4]
- README descreve scanner determinístico de maturidade do repositório: **36 checks, 108 pontos, seis dimensões, níveis L0–L4**, saída JSON/Markdown/badge e integração CI. Dimensões: contexto, skills, hooks, sensores, CI e higiene/segurança. Declara ausência de chamadas LLM e de rede durante o scan. São declarações do projeto, não resultados de execução nesta pesquisa. [H3]
- O próprio README explicita que não mede qualidade dos testes, veracidade/atualidade das regras, correção funcional nem práticas externas ao filesystem. Também admite mudanças do modelo de maturidade em versões minor. A promessa de repetibilidade exige fixar versão/modelo de pontuação e conteúdo efetivamente examinado; o scan inclui arquivos não rastreados e ignorados relevantes. [H3]
- **Uso aplicável:** inventário complementar de infraestrutura do repo. Se integrado futuramente, fixar versão, registrar exclusões e completude do scan e conferir cobertura Rust/Cargo/Tauri. Não usar nota/L4 como certificado de segurança ou probabilidade de sucesso do agente. Uma chamada inicial via `npx` pode baixar o pacote, embora o scanner declare não usar rede: nenhuma execução ou instalação foi realizada aqui.

### Candidato B: `seanrobertwright/harness-score`

- Repositório: <https://github.com/seanrobertwright/harness-score>, licença MIT segundo README e API GitHub. É um **workflow Archon**, não o mesmo CLI do candidato A. [H2, H5]
- README descreve dois nós: detector Bun determinístico com **10 checks e nota de 0–100**, seguido de relatório por **Claude Opus 4.7**. Requer Archon e Claude Code autenticado. Portanto, não herda a propriedade “zero chamadas LLM” do outro projeto. [H5]
- Limitações declaradas: scoring grosseiro, cobertura parcial de ecossistemas e qualidade arquitetural/documentação fora do MVP. Um dos checks pontua a existência do próprio relatório; isso reforça que a nota é uma heurística de prontidão, não eval de desempenho. [H5]

**Conclusão de identidade:** há um candidato consistente com a ferramenta npm de auditoria estática (`paladini/harness-score`) e outro projeto homônimo com arquitetura e escala diferentes (`seanrobertwright/harness-score`). Sem URL/owner adicional do solicitante, a intenção permanece ambígua. Nenhuma evidência consultada os identifica como projeto oficial Anthropic ou OpenAI. O resultado semelhante `ai-harness-scorecard` também é distinto; não foi auditado. [H2–H5]

## 8. Implicações priorizadas

1. **Definir o backend de execução:** começar com um adaptador Codex app-server se o objetivo é incorporar Codex; escolher loop API próprio apenas se o produto precisa controlar o agente em profundidade.
2. **Concentrar autoridade em Rust:** validação IPC, supervisor, políticas, armazenamento e cancelamento; React projeta eventos e responde a interações.
3. **Persistir estado recuperável:** IDs externos, checkpoints e evidências; detectar execução interrompida e reconciliar efeitos antes de repetir.
4. **Fazer isolamento verificável:** workspace separado mais limites reais de filesystem/rede/processos. Capability da WebView, sandbox do agente e worktree resolvem problemas diferentes.
5. **Instrumentar desde o primeiro fluxo:** consumo por tentativa, custo estimado datado, estado final, intervenção humana e falhas.
6. **Construir uma suite pequena e representativa:** testar retomada, cancelamento, aprovações, IPC e efeitos no app desktop. Adotar scanner estático apenas como sinal complementar.

Estas prioridades são síntese de engenharia das fontes, não exigências literais dos fornecedores nem diagnóstico da implementação local atual.

## 9. Fontes efetivamente consultadas

### Anthropic — primárias oficiais

- **[A1]** [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents), 26/11/2025.
- **[A2]** [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents), 29/09/2025.
- **[A3]** [Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents), 09/01/2026.
- **[A4]** [Beyond permission prompts: making Claude Code more secure and autonomous](https://www.anthropic.com/engineering/claude-code-sandboxing), 20/10/2025.
- **[A5]** [Usage and Cost API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api), consultada também pelo [endereço de API redirecionado](https://platform.claude.com/docs/en/api/usage-cost-api).

### OpenAI — primárias oficiais

- **[O1]** [Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/), 11/02/2026.
- **[O2]** [Codex App Server](https://developers.openai.com/codex/app-server), protocolo, ciclo de vida, schemas e transportes. A resposta extensa foi truncada pela ferramenta; os achados deste documento usam as seções efetivamente recebidas sobre integração, inicialização e lifecycle, sem presumir leitura integral de todos os métodos.
- **[O3]** [Codex SDK](https://developers.openai.com/codex/codex-sdk.md), requisitos e exemplos TypeScript/Python.
- **[O4]** [Codex sandboxing](https://developers.openai.com/codex/sandboxing.md), fronteiras e aprovações.
- **[O5]** [Evaluation best practices](https://developers.openai.com/api/docs/guides/evaluation-best-practices.md), práticas e aviso de encerramento da plataforma.
- **[O6]** [Function calling](https://developers.openai.com/api/docs/guides/function-calling.md), fluxo e contrato de ferramentas; resposta extensa parcialmente truncada, mas fluxo, schemas e custos de definições foram recebidos.
- **[O7]** [Conversation state](https://developers.openai.com/api/docs/guides/conversation-state.md), persistência, reprodução e cobrança de histórico.
- **[O8]** [Safety in building agents](https://developers.openai.com/api/docs/guides/agent-builder-safety.md), prompt injection, vazamento e limitações. A página é de Agent Builder, produto em encerramento; princípios de segurança foram usados sem recomendar sua adoção.
- **[O9]** [API pricing](https://developers.openai.com/api/docs/pricing.md), categorias de cobrança; valores não foram congelados como requisito do app.

### GitHub — identidade e documentação dos próprios projetos

- **[H1]** [Busca ampla pela API](https://api.github.com/search/repositories?q=harness-score&per_page=15), resposta parcialmente truncada; usada para descoberta, não inventário completo.
- **[H2]** [Busca por nome pela API, primeiros cinco resultados](https://api.github.com/search/repositories?q=harness-score+in%3Aname&per_page=5), inclui os dois homônimos e metadados de licença.
- **[H3]** [README de paladini/harness-score](https://raw.githubusercontent.com/paladini/harness-score/main/README.md).
- **[H4]** [Manifest do CLI](https://raw.githubusercontent.com/paladini/harness-score/main/packages/cli/package.json) e [manifest do monorepo](https://raw.githubusercontent.com/paladini/harness-score/main/package.json). O monorepo privado para publicação tem versão própria; não confundir com a versão do CLI.
- **[H5]** [README de seanrobertwright/harness-score](https://raw.githubusercontent.com/seanrobertwright/harness-score/main/README.md).

### Stack — fonte primária complementar

- **[T1]** [Tauri v2 — Capabilities](https://v2.tauri.app/security/capabilities/), exposição IPC e limites de proteção.

## Limitações gerais

Pesquisa documental, sem instalar pacotes, executar os agentes de produto, medir custos reais ou testar sandboxes. As páginas e branches `main` são mutáveis e não foram arquivadas/fixadas por commit; versões, preços, flags e disponibilidade precisam ser revalidados na implementação. A pesquisa foi delegada a um agente de pesquisa; isso não constitui teste do runtime Harnessy. Não foi feita auditoria do código-fonte completo dos scanners nem enumeração dos 59 resultados da busca por nome. As recomendações Tauri são propostas para essa stack, não comprovação de que o app atual já as implementa.
