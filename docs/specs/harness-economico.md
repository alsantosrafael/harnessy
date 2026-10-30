# Spec — Harness econômico: núcleo fixo, cinto de utilidades

[← SPEC](../../SPEC.md) · [Plug-and-play](harness-plug-and-play.md) · [Issue #2 (MVP)](https://github.com/alsantosrafael/harnessy/issues/2) · [Progresso](../progress.md)

Versão do documento: 1.6 · 2026-10-30 · Status: proposta. Parte do MVP da issue #2 (detecção de verificações, loop de até 3 ciclos com baseline, guarda do diff, prompt de 24 KB, PR pelo Rust) e acrescenta só o que falta para o Harnessy ser útil em qualquer projeto, barato e confiável. A 1.3 substituiu a 1.2: o que não resolve um problema já observado foi para §9, cada item com o dado que o traz de volta. A 1.4 acrescenta o revisor e a decomposição como Ações (§6.4, §6.5), não como fatias novas, porque o uso manual do próprio fluxo (implementação desta spec por subagentes `haiku`) mostrou que são as peças que mais sustentam a confiança no modelo barato. A 1.5 tira o `haiku` do centro: o harness é medido pelo ganho que dá a **cada** modelo (§1, §8), o modelo vem sempre da escolha do usuário ou da Ação, a escalada é relativa ao modelo escolhido (§7.3), e entram os ganhos sem custo de tokens que servem a qualquer modelo (§7.5). Também registra o estado da implementação em §12.

## 1. Objetivo

O Harnessy pode mudar de tela, de fluxo e de Ações. Duas capacidades não mudam:

1. **Integrar com ferramentas:** runtimes de agente (Claude Code hoje, Codex e outros depois), as verificações do próprio projeto e git/GitHub, por contratos estáveis no núcleo Rust.
2. **Mitigar alucinação:** o resultado vale pelo que o núcleo verificou, nunca pelo que o modelo disse.

Sobre esse núcleo fixo, o Harnessy é um **cinto de utilidades**. Cada opção que o usuário vê é uma Ação declarada em um arquivo curto, que reaproveita os conectores e a verificação. Criar uma opção nova é escrever um arquivo, não mudar código.

Economia é consequência dessas duas coisas: o núcleo faz o trabalho determinístico (detectar, verificar, apontar a falha) e o modelo gasta tokens só no que exige julgamento.

**Agnóstico de modelo.** Nenhum mecanismo do núcleo pressupõe um modelo: falha estruturada, contraprova, prefixo fixo, Ações, ledger e o trait `Runtime` valem igual para `haiku`, `sonnet`, `opus` ou um runtime que não seja Claude. O modelo vem da escolha do usuário ou da Ação; o harness se adapta a ele, nunca o contrário. A meta é que **qualquer modelo** entregue mais tarefas aprovadas e gaste menos tokens por tarefa aprovada com o harness do que sem ele. "Um modelo menor com harness se aproxima de um maior sem harness" é uma consequência a observar em §8, não a meta.

## 2. O que a issue #2 não resolve

| Lacuna | Consequência |
| --- | --- |
| O caminho do Claude Code está espalhado em `run/guard.rs`, `run/stream.rs` e `flow/pipeline`, sem contrato. | Plugar outro runtime exige reescrever o fluxo. |
| O prompt de correção envia os últimos 8 KB da saída (`flow/verify.rs`). | Um modelo pequeno não acha o teste quebrado no meio do ruído e gasta ciclos. |
| Checks verdes não provam que o critério foi atendido. | O agente pode escrever um teste que já passava, ou nenhum teste, e a Run fica "Pronto". |
| O prompt manda o agente rodar as verificações do `AGENTS.md` (`run/prompt.rs`). | Os mesmos checks rodam duas vezes, e o agente gasta turnos na suíte inteira. |
| Custo lido com `unwrap_or(0.0)` (`run/stream.rs`); tokens não gravados por Run. | Custo desconhecido vira zero; não há como comparar modelos. |
| Só existe a Ação "Implementar", fixa em Rust. | Cada utilidade nova (explicar, adicionar testes, corrigir baseline) exige código. |
| Comando negado pelo `guard` em uma Run é tentado de novo na seguinte. | Turnos gastos no mesmo erro em todas as Runs. |

## 3. Arquitetura: o que é fixo e o que é mutável

```
            ┌──────────────── Ações (mutável) ─────────────────┐
            │ implementar · explicar · adicionar-testes · …    │
            │ arquivo .toml: papel, acesso, modelo, prompt     │
            └───────────────┬──────────────────────────────────┘
                            │ compõem
 ┌──────────── Núcleo fixo (Rust) ─────────────────────────────┐
 │ Conectores      Runtime (trait) · Perfil/Checks · Git/GH    │
 │ Verificação     checks · falha estruturada · contraprova    │
 │                 · guarda do diff                             │
 │ Ledger          uso neutro por ciclo · desfecho · decisão   │
 └──────────────────────────────────────────────────────────────┘
```

Regras da fronteira:

- Uma Ação **escolhe** papel, acesso, modelo e o texto da tarefa. Ela **não** escolhe ferramentas, comandos, ambiente, caminhos protegidos nem se a verificação roda. Isso é do núcleo e vale para qualquer Ação.
- O núcleo monta o **início** do prompt (regras fixas, comandos do Perfil, instruções do repositório, mapa). A Ação fornece só o **final** (o pedido). Isso mantém o prefixo estável para o cache de prompt e impede que uma Ação remova uma regra de segurança.
- Nenhum domínio conhece o Claude Code além do adaptador dele.

## 4. Núcleo: conectores

### 4.1 Contrato de runtime

- Trait `Runtime` em `run/runtime.rs`, com:
  - `start(spec: AgentSpec) -> stream de AgentEvent`;
  - `cancel()` (grupo de processos inteiro, como hoje);
  - `capabilities()`, que declara o que o runtime suporta: `resume`, `usage`, `cost`, `json_schema`.
- `AgentSpec` é neutro: diretório, prompt, acesso (`leitura | escrita`), modelo, limite de turnos e tempo, sessão a retomar (opcional).
- `AgentEvent` é neutro: `Text`, `ToolUse`, `Denied { command }`, `Usage(CycleUsage)`, `Done { session_id, summary }`, `Error`.
- `CycleUsage { input, output, cache_read, cache_write, cost_usd, turns }`, com cada campo `Option`. Desconhecido é `None`, nunca zero.
- **Adaptador Claude Code:** move para trás do trait o que já existe em `guard::args`, `guard::settings_json`, `guard::env` e `stream.rs`, sem mudar comportamento. Ele traduz o acesso da Ação em `allow`/`deny`: `leitura` = `Read`, `Glob`, `Grep`; `escrita` = o conjunto atual mais os comandos do Perfil.
- Plugar um runtime novo = um adaptador novo + a sonda em `connections`. Nada em `flow`, `action` ou no frontend muda.

### 4.2 Verificações do projeto

- Continua o Perfil da issue #2 (`.harnessy/workspace.toml`, aprovação local por hash).
- **Typecheck obrigatório por padrão:** quando a detecção encontra typecheck (`tsc`/script `check`/`typecheck`, `cargo check`, `go vet`, mypy ou pyright configurados), ele é proposto como obrigatório. É a defesa mais barata contra API e import inventados.
- Os comandos do Perfil entram na lista `allow` do agente. O prompt diz que **o núcleo roda as verificações ao final** e que o agente pode rodar os comandos listados se quiser conferir antes. A instrução "rode as verificações do `AGENTS.md`" sai do prompt.

### 4.3 Git e GitHub

Sem mudança em relação à issue #2: worktree por Run, commit, push e PR pelo Rust, nunca pelo agente.

## 5. Núcleo: verificação

### 5.1 Falha estruturada

- Módulo puro `flow/failure.rs`: saída de um check → `Vec<Failure { check, file, line, test_name, expected, actual, excerpt }>`. O parser é escolhido pelo comando do check:

  | Runner | Sinais |
  | --- | --- |
  | Vitest / Jest | `FAIL <arquivo>`, `✕ <nome>`, `AssertionError`, `Expected:` / `Received:` |
  | `cargo test` | `test <nome> ... FAILED`, `panicked at <arquivo>:<linha>:<col>`, `left:` / `right:` |
  | `cargo check` / `clippy` | `error[Exxxx]`, `--> <arquivo>:<linha>:<col>` |
  | `tsc` | `<arquivo>(<linha>,<col>): error TSxxxx` |
  | pytest | `FAILED <arquivo>::<nome>`, `E   assert`, `<arquivo>:<linha>:` |
  | `go test` | `--- FAIL: <nome>`, `<arquivo>:<linha>:` |
  | fallback | uma `Failure` por check com `excerpt` dos últimos 4 KB (o comportamento de hoje) |

- O prompt de correção traz uma seção por falha (arquivo:linha, teste, esperado, obtido, trecho de até 1 KB). Com mais de 5 falhas, as 5 primeiras em detalhe e a contagem do resto. Total de até 8 KB.
- A guarda do diff da issue #2 produz `Failure` no mesmo formato e entra no mesmo prompt.
- Cada `Failure` é gravada em `check_results.failures` (JSON) para a interface.
- Nunca usa modelo para interpretar falha.

### 5.2 Contraprova do teste

Checks verdes provam que nada quebrou. A contraprova mostra se algum teste **prova a mudança**: um teste que só passa com ela.

- Roda depois que a Run fica verde, sem tokens, no próprio worktree (dependências já instaladas):
  1. O núcleo já commitou o resultado (issue #2). Separa os arquivos alterados em **testes** (padrões do runner: `*.test.*`, `*_test.go`, `tests/`, `test_*.py`, módulos `#[cfg(test)]` em arquivo próprio) e **código**.
  2. Restaura só os arquivos de código para o SHA base (`git checkout <base> -- <código>`) e roda os checks de teste.
  3. Restaura o commit da Run (`git checkout HEAD -- .`) e confere que o worktree voltou ao SHA da Run.
- Cada teste novo ou alterado recebe um veredito, usando os nomes extraídos por §5.1:
  - **prova:** falha (ou não compila) sem a mudança;
  - **não prova:** passa mesmo sem a mudança.
- No Resultado:
  - "Pronto · comprovado por N testes";
  - "Pronto · nenhum teste prova a mudança" (aviso destacado);
  - "Pronto · sem testes alterados" (aviso).
- Por padrão é **aviso, não bloqueio**, como os outros avisos da issue #2. Uma Ação pode declarar `exigir_prova = true`: aí "nenhum teste prova" vira uma `Failure` (`check = "contraprova"`) e consome um ciclo de correção.
- Pulada, com motivo visível, quando o Perfil não tem check de testes ou quando a Ação declara `contraprova = false` (por exemplo, "adicionar testes" a código existente, em que os testes devem passar no base).
- Falha de infraestrutura na contraprova (timeout, restauração inconsistente) não altera o desfecho: vira aviso "contraprova não concluída" e preserva o worktree.

### 5.3 Guarda do diff

Sem mudança em relação à issue #2: caminhos protegidos, segredos, limite de tamanho e sinais de teste enfraquecido.

## 6. Cinto: Ações declarativas

### 6.1 Formato

Arquivo TOML, um por Ação:

```toml
schema = 1
id = "adicionar-testes"
nome = "Adicionar testes"
descricao = "Cobre com testes o comportamento descrito na tarefa, sem mudar código de produção."
papel = "implementador"      # define o runtime pelo mapeamento de src/lib/roles.ts
acesso = "escrita"           # leitura | escrita
# modelo = "sonnet"          # opcional; sem ele, o modelo escolhido pelo usuário
# esforco = "medium"         # opcional; repassado ao runtime quando ele suporta (§7.5)
# max_turns = 15             # opcional; teto de turnos desta Ação, até o limite do núcleo
contraprova = false          # padrão: true em escrita
exigir_prova = false         # padrão: false
pedido = """
Escreva testes para o comportamento abaixo. Não altere arquivos fora de testes.

{{tarefa}}

Critérios:
{{criterios}}

Comece por: {{arquivos_provaveis}}
"""
```

- **Variáveis disponíveis** (conjunto fechado, preenchido pelo núcleo): `{{tarefa}}`, `{{descricao}}`, `{{criterios}}`, `{{arquivos_provaveis}}`. Ações com `alvo = "run"` (que agem sobre o resultado de uma Run, não sobre a Tarefa) também recebem `{{diff}}` (orçamento de 16 KB, cortado por arquivo), `{{checks}}` (nome e resultado de cada check) e `{{contraprova}}` (veredito de §5.2). Variável desconhecida, ou de Run em Ação com `alvo = "tarefa"`, invalida o arquivo no carregamento, com mensagem que cita a linha.
- **Saída** (`saida`, conjunto fechado interpretado pelo núcleo): `texto` (padrão; exibido como opinião), `revisao` (§6.4) ou `passos` (§6.5). As saídas estruturadas são pedidas ao runtime por schema (`--json-schema` no Claude Code); runtime sem `json_schema` não executa essas Ações e a interface diz por quê. Saída inválida não altera a Run e aparece como "o agente não devolveu o formato esperado".
- **Garantido pelo núcleo, sem campo na Ação:**
  - prefixo do prompt (§3);
  - ferramentas por nível de acesso (§4.1);
  - ambiente fixo (regra 26);
  - worktree próprio para escrita;
  - todos os checks obrigatórios, guarda do diff e loop de correção para escrita;
  - limites de turnos, tempo e saída.
- **Leitura:** sem worktree de escrita e sem checks. O resultado é texto, exibido como **opinião do agente**, nunca como evidência, e não move o card.

### 6.2 Onde vivem

| Origem | Caminho | Aprovação |
| --- | --- | --- |
| Embutidas | compiladas no app (`implementar`, `explicar`, `revisar`, `dividir-em-passos`) | — |
| Do usuário | pasta de dados do app, `actions/*.toml` | Criadas pela interface ou à mão; valem em todos os workspaces |
| Do repositório | `.harnessy/actions/*.toml` | Por hash, como o Perfil (SPEC §15). Alterar o arquivo exige nova confirmação |

Precedência por `id`: repositório > usuário > embutida. A embutida `implementar` passa a ser escrita nesse formato, para que o formato seja exercitado desde o primeiro dia.

### 6.3 Na interface

- A ação principal do card continua decidida pelo estado (issue #2).
- As demais Ações aparecem em **Mais ações** na Tarefa, com nome e descrição.
- **Nova Ação** abre um formulário (nome, acesso, modelo, pedido com as variáveis clicáveis) e grava o TOML na pasta do usuário. O usuário não precisa escrever TOML.
- O prompt exato continua visível antes de iniciar (issue #2), agora com o prefixo do núcleo e o pedido da Ação separados.

### 6.4 Revisor: Ação de leitura sobre a Run

Revisão independente sem fatia nova: é a Ação embutida `revisar`, no formato de §6.1.

```toml
schema = 1
id = "revisar"
nome = "Revisar resultado"
papel = "revisor"            # runtime pelo mapeamento de src/lib/roles.ts
acesso = "leitura"
alvo = "run"
# modelo omitido: um degrau acima do modelo da Run na lista de §7.3 (o mesmo, se já no topo)
saida = "revisao"
quando = "risco"             # risco | sempre | manual
pedido = """
Revise a mudança abaixo contra a tarefa e os critérios. Aponte só defeitos concretos.
{{tarefa}}
{{criterios}}
{{checks}}
{{contraprova}}
{{diff}}
"""
```

- **Entrada:** só tarefa, critérios, diff, resultado dos checks e veredito da contraprova. Nunca a conversa, a sessão ou o resumo do implementador; o revisor roda em sessão nova.
- **Quando roda:** só com a Run em "Pronto"; nunca paga para revisar o que os checks já reprovaram. Com `quando = "risco"` (padrão), roda só se houver sinal: contraprova "não prova" ou "sem testes alterados", aviso da guarda do diff, ou diff acima de 300 linhas. `sempre` roda em toda Run verde; `manual` só pelo botão **Revisar resultado**.
- **Saída `revisao`:** `{ "veredito": "approve" | "request_changes", "achados": [{ "arquivo", "linha", "severidade": "bloqueia" | "sugere", "texto" }] }`, até 10 achados.
- **Filtro contra alucinação do revisor (determinístico):** o núcleo descarta todo achado cujo `arquivo` não está no diff ou cuja `linha` não pertence a um trecho alterado (com 3 linhas de contexto). A contagem de descartados fica no ledger e aparece em **Detalhes técnicos**. `request_changes` sem nenhum achado `bloqueia` restante vira `approve`, com a nota "achados descartados por estarem fora do diff".
- **Efeito:** `request_changes` vira uma `Failure` por achado `bloqueia`, no formato de §5.1 (`check = "revisão"`, `file`, `line`, `excerpt` = texto), e dispara um ciclo de correção normal (`trigger = review`), dentro do limite de 3 ciclos. Depois da correção, checks e contraprova rodam de novo; o revisor roda de novo só se houver novo sinal de risco. Achados `sugere` aparecem no Resultado e não geram ciclo.
- Revisor por Codex, como previsto na SPEC, é um adaptador novo do trait `Runtime` mais a troca do runtime do Papel `revisor`; nada nesta Ação muda.

### 6.5 Decomposição: tarefas encadeadas

Sem motor de passos, sem checkpoint e sem estado novo de Run.

- Ação embutida `dividir-em-passos`: `acesso = "leitura"`, `alvo = "tarefa"`, `saida = "passos"`. Devolve até 5 subtarefas `{ "titulo", "criterios": [...] }`, em ordem.
- O usuário edita e confirma; cada subtarefa vira uma **Tarefa normal** no quadro, com referência à Tarefa de origem e à anterior da cadeia (`tasks.parent_id`, `tasks.after_id`, anuláveis).
- **Única mudança de núcleo:** uma Run pode partir da **branch de uma Run anterior** em vez do ramo base. Ao iniciar uma Tarefa com `after_id`, o Harnessy propõe como base a branch da última Run "Pronto" da Tarefa anterior, com o rótulo "continua de: <título>". O usuário pode trocar para o ramo base. A guarda do diff, a contraprova e o revisor comparam com a base escolhida, não com o `main`.
- Cada passo é disparado pelo usuário. Não há execução automática em sequência.
- O PR de uma Tarefa encadeada aponta para a branch da anterior enquanto ela não estiver mergeada, e o corpo do PR cita a cadeia.

## 7. Economia mínima

### 7.1 Ledger

Migração em `db.rs` acrescentando a `runs`:

- `action_id`, `model`;
- `cycles`, `turns`;
- `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_write_tokens`, `cost_usd` (somados de `CycleUsage`; `NULL` se algum ciclo não informou);
- `prepare_ms`, `agent_ms`, `checks_ms`;
- `outcome` (estados da issue #2);
- `proof` (`proved | not_proved | no_tests | skipped`);
- `review` (`approve | request_changes | skipped`, `NULL` quando o revisor não rodou) e `review_discarded` (achados descartados pelo filtro de §6.4);
- `attempt` (ordem da Run entre as Runs da mesma Tarefa; `attempt > 1` é Run refeita, o sinal de intervenção humana que o resto do ledger não captura);
- `base_run_id` (Run cuja branch serviu de base, §6.5; `NULL` para o ramo base);
- `accepted` (`1` ao publicar ou aceitar, `0` ao descartar, `NULL` sem decisão).

Mais a tabela `run_cycles` (`run_id`, `cycle`, `model`, `session_id`, `resumed`, tokens, `cost_usd`, `trigger` ∈ `initial | check_failed | diff_guard | proof | review | bigger_model`). A chamada do revisor também é uma linha de `run_cycles` (com `trigger = review` e o modelo do revisor), para que seu custo entre no total da Run.

Com isso, toda intervenção fica contável: ciclos de correção, revisões, modelo maior pedido pelo usuário, tentativas manuais ("Tentar mais uma correção") e Runs refeitas.

- Tokens são a métrica primária; USD aparece quando o runtime informa, rotulado como referência em conta de assinatura.
- O detalhe da Run mostra modelo, ciclos, tokens, custo ("não informado" quando `NULL`) e durações. Sem painel agregado nesta spec.

### 7.2 Continuidade entre ciclos

Duas regras, sem heurística de tempo:

1. Mesmo modelo e o runtime declara `resume`: o ciclo de correção retoma a sessão e envia **só** as falhas de §5.1 e a instrução de não enfraquecer testes.
2. Modelo diferente, runtime sem `resume` ou retomada recusada antes do primeiro evento: sessão nova com o prompt completo e as falhas. A recusa gera o status "sessão não retomada; contexto reenviado" e não conta como ciclo.

### 7.3 Modelo maior por decisão do usuário

- Uma **lista ordenada de modelos**, do mais leve ao mais forte, é configuração local do app (padrão: a ordem de `guard::MODELS` por capacidade, hoje `haiku → sonnet → opus → fable`). Ela não define o modelo inicial: a Run começa no modelo escolhido pelo usuário ou na Ação.
- Em **Precisa de você**, além das ações da issue #2, **Tentar com `<próximo>`**: o próximo modelo da lista acima do modelo do último ciclo, com o nome no botão. Se o modelo atual já é o último da lista (ou não está nela), o botão não aparece.
- Abre um ciclo extra em sessão nova com tarefa, falhas atuais e `git diff --stat` da Run (`trigger = bigger_model`). É escolha do usuário, visível, e fica registrada no ledger como dado para a escada automática (§9).

### 7.4 Comandos negados lembrados

- Tabela `workspace_denials` (`workspace_id`, `command`, `count`, `last_seen`), alimentada pelos eventos `Denied` de qualquer runtime.
- O prefixo do prompt das próximas Runs do workspace ganha a linha "Não tente: `curl`, `gh`… (negados pelo Harnessy)", com até 20 comandos, os mais frequentes primeiro.
- Fato determinístico: nada é texto do modelo.

### 7.5 Ganhos sem custo de tokens, para qualquer modelo

- **Arquivos prováveis de verdade:** `{{arquivos_provaveis}}` deixa de ser "(não calculado)". O núcleo extrai termos do título e dos critérios (palavras com 4+ letras, sem stopwords em português e inglês, mais identificadores em `camelCase`/`snake_case`), casa com nomes de arquivo (`git ls-files`) e com conteúdo (`git grep -l -i`), e devolve até 10 caminhos ordenados por número de termos casados, só caminhos, sem conteúdo. Sem casamento, a variável vira "(nenhum encontrado)". Custa milissegundos e reduz turnos de exploração.
- **Instruções sem duplicação:** um teste de integração (ignorado por padrão, roda contra a CLI instalada) verifica se o runtime já carrega `CLAUDE.md`/`AGENTS.md` do worktree com os argumentos atuais. O resultado decide o que o prefixo do núcleo faz: se carrega, o prefixo não injeta o conteúdo; se não carrega, injeta o trecho orçamentado (4 KB). O teste registra o achado em `docs/development.md`.
- **Esforço e teto de turnos por Ação:** campos opcionais `esforco` e `max_turns` (§6.1). São configuração, não lógica: `esforco` vai para o runtime quando ele declara suporte (`--effort` no Claude Code) e é omitido caso contrário; `max_turns` nunca passa do limite do núcleo (`guard::MAX_TURNS`). O ledger já grava o modelo; passa a gravar também `effort`.
- **Comandos negados lembrados** (§7.4) e **revisor só com sinal de risco** (§6.4, `quando = "risco"`) completam o conjunto: ambos cortam turnos ou chamadas só quando a evidência pede.

## 8. Medição

- `experiments/economico/`: as 8 tarefas atuais de `experiments/loop/tasks` como sanidade, mais pelo menos 10 tarefas reais com alteração em vários arquivos em um repositório-fixture (bug, feature pequena, refactor, critério ambíguo, teste quebrado no baseline, tentação de enfraquecer teste).
- Cada tarefa tem **testes ocultos** que o agente não vê. "Aprovada" = testes ocultos verdes. É o que mede alucinação verde: checks visíveis passam, ocultos não.
- Células em **pares por modelo**: para cada modelo da lista de §7.3 (pelo menos `haiku`, `sonnet` e `opus`), o modelo **cru** (prompt de hoje, sem loop) e o mesmo modelo **com harness**. Uma célula extra por modelo, **com harness + revisor** (`quando = "risco"`, um degrau acima), responde se a chamada extra se paga em aprovação. 3 trials por célula.
- Métricas por célula: taxa de aprovação; taxa de "Pronto" (a distância entre as duas mede alucinação); tokens e custo por tarefa aprovada; ciclos; veredito da contraprova; duração p50.
- **Ganho do harness por modelo** = (com harness) − (cru) no mesmo modelo, em aprovação e em tokens por tarefa aprovada. É o número que importa.
- Resultado em `experiments/economico/RESULTADOS.md` com data, versão do back e da CLI.
- **Tese comprovada** quando, para **cada** modelo medido, o harness não reduz a taxa de aprovação e reduz os tokens por tarefa aprovada, e para pelo menos um modelo aumenta a aprovação acima da variação entre trials. A comparação entre modelos diferentes (ex.: menor com harness contra maior cru) fica registrada como observação, sem gate. Até lá, README e interface não afirmam nada sobre economia.

## 9. Quando os dados pedirem

Fora desta spec. Cada item volta só quando o ledger ou §8 mostrar o gatilho.

| Item | Gatilho |
| --- | --- |
| Escada automática de modelos | "Tentar com modelo maior" fecha verde em ≥ 50% das vezes em que é usado, em ≥ 20 Runs |
| Retomar ou recomeçar por tempo de cache, repasse compacto | Ciclos retomados com `cache_read` abaixo de 50% da entrada |
| Esforço escolhido pelo núcleo (em vez de configurado na Ação) | §8 com variação de esforço mostrando ganho consistente de aprovação por token num mesmo modelo |
| Planejador por modelo maior | Runs falhas com muitos turnos de leitura antes da primeira edição |
| Memória de arquivos tocados e correções anteriores | ≥ 30 Runs no workspace e a mesma `Failure` em ≥ 2 Runs |
| Setup persistido (mapa, referências ao harness existente, relatório de prontidão) | Montagem do contexto lenta ou primeira Run com exploração muito acima das seguintes |
| Âncora em duas fases com teste congelado | Contraprova "não prova" ou teste enfraquecido em ≥ 20% das Runs verdes |
| Worktree quente (clonefile de dependências) | `prepare_ms` > 20% da duração média da Run |
| Painel de custo e aceitação no board | Dois ou mais modelos em uso com ≥ 20 Runs cada |
| Revisor em toda Run verde (`quando = "sempre"` como padrão) | Runs aprovadas sem revisão (sem sinal de risco) descartadas pelo usuário em ≥ 15% dos casos |
| Motor de passos (checkpoint, execução automática em sequência) | Cadeias de ≥ 3 Tarefas encadeadas frequentes, com o usuário só disparando o próximo passo sem editar |
| Captura global de tarefa | Pedido de uso |

## 10. Invariantes (não deve)

- Aceitar a mensagem do agente como prova; "Pronto" depende só de checks, guarda e, quando exigida, contraprova.
- Deixar uma Ação ampliar ferramentas, comandos, ambiente ou caminhos, ou desligar a verificação de uma Ação de escrita.
- Usar modelo para interpretar falha, julgar critério ou gerar memória. O revisor (§6.4) opina; seus achados só viram `Failure` depois do filtro determinístico contra o diff.
- Dar ao revisor a conversa, a sessão ou o resumo do implementador.
- Gravar custo ou tokens desconhecidos como zero.
- Conhecer um runtime fora do adaptador dele.
- Afirmar economia sem o registro de §8.

## 11. Riscos

| Risco | Mitigação |
| --- | --- |
| Parser de falha erra arquivo ou linha | `excerpt` sempre acompanha; fixtures reais por runner; fallback igual ao comportamento de hoje. |
| Contraprova classifica errado (teste e código no mesmo arquivo, como `#[cfg(test)]` inline em Rust) | Arquivo misto conta como código; o teste é avaliado pelo nome e cai em "sem testes alterados" se não houver arquivo de teste; o veredito é aviso, não bloqueio, salvo `exigir_prova`. |
| Contraprova deixa o worktree inconsistente | Roda só depois do commit da Run; confere o SHA ao final; falha preserva o worktree e vira aviso. |
| Ação de repositório maliciosa | Mesma aprovação por hash do Perfil; a Ação não controla ferramentas, ambiente nem verificação. |
| `--resume` muda entre versões da CLI | Recuo para sessão nova (§7.2), testado com sessão apagada. |
| Revisor inventa defeito ou gera ciclo inútil | Filtro de achados contra o diff; só `bloqueia` gera ciclo; ciclo de revisão conta no limite de 3; célula com revisor em §8. |
| Revisor encarece toda Run | Só roda com Run verde e, por padrão, só com sinal de risco; entrada é o diff, não o repositório. |
| Tarefa encadeada parte de branch desatualizada | A base proposta é a última Run "Pronto" da anterior, sempre visível e trocável; a guarda compara com a base escolhida. |
| Runtime sem `usage` | Campos `NULL`; §8 falha se mais de 10% das Runs ficarem sem tokens. |

## 12. Ordem de entrega e definição de pronto

Pré-requisito: issue #2. Cada passo é entregável sozinho.

**Estado em 2026-10-30** (implementado no working tree; `cargo test` 126 verdes, `verify:rust` e `check:quick` verdes; smoke desktop e runs reais com o `claude` ainda não feitos):

| Passo | Estado | Diferenças em relação ao critério de pronto |
| --- | --- | --- |
| 1 | Feito (Back 0.5.0) | — |
| 2 | Feito (Back 0.6.0) | Falhas gravadas como evento `failures` da Run, sem tabela própria; saídas acima de 256 KB perdem o final (`process::drain`). |
| 3 | Feito (Back 0.7.0, 0.11.0) | Detecção de Python só para typecheck (mypy/pyright), sem pytest; testes inline do Rust contam como código. |
| 4 | Feito (Back 0.6.0, 0.10.0; Front 0.5.0, 0.9.0) | — |
| 4b | Feito (Back 0.9.0, Front 0.8.0) | Revisão roda uma vez por run; sem botão manual "Revisar resultado". |
| 4c | Feito (Back 0.9.0, Front 0.8.0) | PR encadeado (base na branch anterior) depende da publicação (F5), ainda inexistente. |
| 5 | Feito (Back 0.8.0, Front 0.7.0) | Lista de modelos é constante (`guard::LADDER`), não configuração da interface. |
| 5b | Feito (Back 0.8.0) | Teste de carregamento do `CLAUDE.md` existe, mas ainda não foi executado. |
| 6 | Parcial | Suíte e executor prontos com 4 tarefas reais (de 10) e testes ocultos conferidos; nenhuma medição registrada. |

1. **Contrato de runtime e ledger** (§4.1, §7.1). Refactor do caminho atual para o trait, sem mudar comportamento; Runs gravam modelo, ciclos, tokens e custo (`NULL` quando desconhecido). *Pronto quando:* teste do adaptador com stream-json fixo, com e sem `usage`; Run com dois ciclos grava duas linhas em `run_cycles` e a soma em `runs`; um runtime falso que implementa o trait roda a Run de ponta a ponta nos testes de integração da issue #2.
2. **Falha estruturada** (§5.1). *Pronto quando:* fixtures reais de cada runner produzem `file` e `line` corretos; saída não reconhecida produz uma `Failure` com `excerpt`; o prompt de correção da fixture com bug cabe em 8 KB e cita arquivo e linha.
3. **Contraprova e typecheck por padrão** (§5.2, §4.2). *Pronto quando:* agente falso que adiciona um teste que falha no base gera "comprovado"; um teste que já passava gera "nenhum teste prova"; com `exigir_prova = true`, o segundo caso consome um ciclo; o worktree termina no SHA da Run em todos os casos; a detecção propõe typecheck obrigatório em fixtures Bun, Cargo e uv.
4. **Ações declarativas** (§6). *Pronto quando:* `implementar` roda a partir do formato TOML; uma Ação de leitura (`explicar`) roda sem worktree de escrita e sem checks; uma Ação de repositório alterada exige nova confirmação; variável desconhecida é rejeitada com a linha; criar uma Ação pelo formulário a faz aparecer em **Mais ações** sem reiniciar o app.
4b. **Revisor** (§6.4, depende de 4 e de `--json-schema` no adaptador). *Pronto quando:* revisor falso que aponta linha fora do diff tem o achado descartado e contado; `request_changes` com achado válido gera um ciclo com `trigger = review` e a `Failure` no prompt de correção; com `quando = "risco"`, Run verde sem sinal de risco não chama o revisor; o prompt do revisor não contém nada da sessão do implementador (teste do montador).
4c. **Tarefas encadeadas** (§6.5). *Pronto quando:* `dividir-em-passos` falso devolve 3 subtarefas que viram Tarefas com `parent_id` e `after_id`; a Run da segunda parte da branch da Run "Pronto" da primeira, com `base_run_id` gravado; a guarda do diff e a contraprova usam essa base; trocar para o ramo base funciona.
5. **Continuidade, modelo maior e comandos negados** (§7.2–§7.4). *Pronto quando:* o ciclo 2 retomado envia só as falhas; apagar a sessão da CLI entre ciclos conclui a Run pelo recuo, com status visível e ciclo não contado; **Tentar com `<próximo>`** abre um ciclo em sessão nova com o modelo seguinte ao do último ciclo e `trigger = bigger_model`, e não aparece quando o modelo atual é o último da lista; um `curl` negado em uma Run aparece em "Não tente" na seguinte.
5b. **Ganhos para qualquer modelo** (§7.5). *Pronto quando:* tarefa com termos que casam com nomes e conteúdo de arquivos de um repositório-fixture recebe esses caminhos em `{{arquivos_provaveis}}`, no máximo 10, sem conteúdo; o teste de duplicação de instruções existe e seu resultado está registrado; uma Ação com `esforco` e `max_turns` produz os argumentos correspondentes no adaptador, `max_turns` acima do limite é recusado na validação, e `runs.effort` é gravado.
6. **Medição** (§8). *Pronto quando:* `experiments/economico/RESULTADOS.md` existe com os pares cru/com harness (e a célula com revisor) para cada modelo medido e o ganho do harness calculado por modelo.

Em todos os passos: `bun run verify`, `bun run verify:rust`, `cargo test` e, quando tocar IPC ou lifecycle, o smoke desktop; `docs/progress.md`, README e `GLOSSARY.md` (termos novos: Contraprova, Runtime) atualizados na mesma mudança que entrega o passo.
