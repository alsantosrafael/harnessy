# Changelog

O produto segue SemVer. Durante 0.x, incompatibilidades exigem uma versão minor e migração explícita.

## Back [0.17.1] — 2026-10-31

- A Ação embutida `revisar` agora é um **staff engineer**: avalia simplicidade e complexidade proporcional à tarefa, fundamentos, aderência às regras e à arquitetura do projeto (lê `AGENTS.md`, `.claude/rules` e `docs/adr`), testabilidade, manutenção e o impacto no usuário. Bugs e casos de borda deixam de ser o foco dela: são do QA, papel separado que entra com o ambiente de teste. O filtro do núcleo continua descartando achados fora das linhas alteradas.

## Front [0.20.0] — 2026-10-31

O quadro passa a mostrar a entrega de cada tarefa:

- Cada card em **Revisão** ganha um selo de estágio, com ícone e texto: **Só local**, **PR em draft**, **PR aberto**, **Checks rodando**, **Checks falhando**, **Conflito**, **Mudanças pedidas**, **Pronto para merge**, **Mergeado** ou **PR fechado**. O tooltip e o leitor de tela dizem o que está acontecendo e qual é a próxima ação.
- O cabeçalho de cada raia resume os estágios, com o que pede ação primeiro (por exemplo, "1 Checks falhando · 2 Pronto para merge").
- Novos comandos rápidos **Publicar PR** (vira **Atualizar PR** depois) e **Fazer merge**, que pede confirmação e explica o motivo quando está indisponível.
- Os PRs abertos são relidos no GitHub ao voltar para a janela e a cada 90 segundos. Uma mudança de estágio é anunciada para leitor de tela sem mover o foco. O ícone de "checks rodando" respeita `prefers-reduced-motion`.

## Back [0.17.0] — 2026-10-31

Entrega de uma run como PR (fatia 1; a interface vem na sequência):

- Novo domínio `delivery` e tabela `deliveries` (migração nova): PR da run, estado, draft, checks, conflitos e o SHA que a revisão aprovou.
- Comandos `publish_run`, `refresh_delivery`, `merge_delivery` e `list_deliveries`. Publicar verifica o diff (recusa diff vazio e arquivos que parecem segredos), faz `git push` sem force e sem hooks, abre o PR **em draft** com tarefa, critérios, contraprova e revisão no corpo, e não duplica se já existir um PR da branch.
- O merge é squash, só se o PR estiver aberto, sem checks reprovados ou pendentes, sem conflito e sem revisão pedindo mudanças; usa `--match-head-commit`, então só mescla o commit que acabou de ser lido. Ao mesclar, a tarefa vai para Concluído.
- Os comandos são assíncronos e não seguram o banco durante a chamada de rede.
- Um novo commit no PR invalida a aprovação da revisão (`reviewed_sha` ≠ `head_sha`).

## Front [0.19.0] — 2026-10-31

- A criação de tarefa na raia **A fazer** ganhou um formulário mais claro: o campo **Título** (com o botão **Criar**) e, embaixo, **+ Descrição e critérios** abre descrição e critérios de aceite opcionais no mesmo lugar. Uma linha diz que só o título basta e que o resto pode vir depois. Ctrl/⌘+Enter cria a partir da descrição ou dos critérios.

## Front [0.18.0] — 2026-10-31

Barra lateral organizada por workspace:

- Cada workspace é um grupo expansível com **Tarefas**, **Chat**, **Conexões** e **Nova tarefa**; clicar numa ação já troca de workspace e de tela.
- O workspace atual fica sempre aberto e mostra o contador de runs em andamento. Sem workspaces, a barra mantém Tarefas e Conexões.

## Front [0.17.1] — 2026-10-30

Correções do QA de regras de negócio:

- O campo de ajuste avisa quando passa de 4.000 caracteres e não envia (antes o texto era recusado só no núcleo).
- O ajuste continua da run **concluída mais recente que deixou branch**, mesmo quando a última run da conversa falhou. Antes partia do código limpo.
- Excluir uma tarefa limpa o cache da conversa dela: o SQLite reaproveita ids, e uma run nova herdava a conversa da excluída.
- Uma resposta de leitura concluída (sem branch e sem aviso) não esconde mais o bloqueio de uma run de escrita anterior.
- O pedido do usuário não é mais cortado junto com o começo de um log longo (acima de 2.000 eventos).

## Back [0.16.1] — 2026-10-30

Correções do QA de regras de negócio:

- **Ajuste longo travava o workspace:** `start_run` criava a run, registrava e movia o card antes de validar o ajuste; ao recusar, ficava uma run `preparing` sem processo. O prompt (e a validação) agora vem antes de criar a run.
- O texto do ajuste só afirma que "o trabalho anterior já está neste worktree" quando a run parte da branch de outra run.
- `review_run` recusa uma segunda revisão da mesma run ("Esta run já está sendo revisada"): antes duas disputavam o slot de cancelamento e o número do ciclo, e um dos ciclos se perdia.
- Na abertura do app, runs interrompidas de Ações de leitura do repositório não mexem mais no card. A run de escrita grava o revisor ao nascer; a de leitura não, e é isso que as distingue da lista fixa.
- A conversa explica quando a revisão não roda porque está desligada ("Nunca" ou Ação em modo manual), e a mensagem de revisão dispensada só diz "comprovada" quando a contraprova de fato comprovou.
- Excluir uma tarefa remove também os worktrees que as runs dela deixaram no disco (as de runs que falharam). As branches continuam no git.

## Front [0.17.0] — 2026-10-30

- No **Chat**, a conversa agora é da **tarefa**, não de uma run só: ela reúne todas as runs, em ordem, cada uma com uma divisória (`Run #n · estado`), o resumo e a contraprova das anteriores. Pedir um ajuste não apaga mais o que já foi dito.
- O que você pediu aparece na conversa, à direita: o texto da tarefa na primeira run e o ajuste digitado nas seguintes.
- A lista da esquerda tem **uma linha por tarefa** (com "n runs"), que não muda de grupo quando a run termina; a conversa aberta fica sempre visível, mesmo fora das mais recentes.
- O log mostrado por run passou de 500 para 2.000 eventos, para não cortar o começo de runs longas.

## Back [0.16.0] — 2026-10-30

- Cada run grava, como primeiro evento, o pedido do usuário (`user`): o ajuste digitado ou o título e a descrição da tarefa.
- Corrigido: um ajuste que falha, termina sem alterações ou é cancelado/interrompido devolvia o card para **A fazer**, mesmo com trabalho pronto de uma run anterior. Agora o card volta para **Revisão** quando a tarefa já tem uma run concluída, e só vai para A fazer se não tiver.

## Front [0.16.1] — 2026-10-30

- Corrigido: **Excluir** (e os avisos de "descartar alterações") não faziam nada no app. Eles usavam o `window.confirm` do navegador, que na janela do macOS pode devolver "não" sem mostrar nada. Agora a confirmação é um diálogo do próprio app, com **Cancelar** em foco e o botão de perigo em vermelho; Esc fecha.

## Front [0.16.0] — 2026-10-30

- Ao escolher uma tarefa, o **detalhe aparece embaixo do quadro**, na largura toda e em duas colunas (edição à esquerda; agente, ações, prompt e histórico à direita), em vez de numa coluna ao lado. O quadro continua visível em cima, com rolagem própria em cada raia, e a tela desce até o detalhe.
- Ao lado do quadro, uma coluna de **comandos rápidos** da tarefa: Implementar, Revisar última run, Abrir conversa, Copiar branch, Concluir (ou Reabrir) e Excluir. Cada comando indisponível diz o motivo em texto. A escolha de continuar da branch do passo anterior vale para o botão do detalhe e para o comando rápido.
- Em janelas estreitas (< 900 px) os comandos rápidos e o detalhe descem para baixo do quadro.

## Front [0.15.0] — 2026-10-30

- Cada card ganhou **editar** (abre a tarefa com o cursor no título) e **excluir** (com confirmação), ao lado de mover; o painel da tarefa também tem o ícone de excluir.
- O aviso de bloqueio ficou claro: título conforme o motivo (sem alterações, run falhou, ações negadas), uma frase dizendo o que **Dispensar aviso** faz (só esconde o aviso, não muda o estado) e, quando a run terminou sem alterações, o botão **Concluir tarefa** para quando o trabalho já existe. Dispensar mostra uma confirmação.
- **Revisar agora** (na conversa da run) e **Revisar a última run agora** (no painel da tarefa) disparam a revisão à mão, com o revisor escolhido, mesmo sem sinal de risco. O botão indica "Revisando…" até o veredito.

## Back [0.15.0] — 2026-10-30

- Novo comando `delete_task(workspaceId, taskId)`: exclui a tarefa, o histórico e as runs (eventos e ciclos) numa transação; recusa com run em andamento; passos que dependiam dela perdem só o vínculo. As branches continuam no git.
- Novo comando `review_run(runId, reviewer, reviewerModel)`: revisão sob demanda de uma run concluída, sobre o diff da branch (`base..branch`, já que o worktree é removido). Grava veredito e ciclo; o estado da run e o card não mudam. Recusa outra run em andamento.

## Front [0.14.0] — 2026-10-30

- No **Chat**, com uma run aberta o campo vira **Pedir ajuste**: o texto continua na mesma tarefa, partindo da branch da run concluída, em vez de criar uma tarefa nova. **Nova tarefa** (ao lado do botão) volta ao modo de criação. Com a run em andamento, o campo explica que é preciso esperar.
- O padrão de revisão passou a **Sempre**. Quem já tinha "Só com risco" salvo mantém a escolha.

## Back [0.14.0] — 2026-10-30

- `start_run` recebe `instruction` opcional (ajuste pedido numa run encadeada, até 4.000 caracteres), anexado depois do pedido da Ação. Compatível: sem o argumento, nada muda.
- Quando a revisão não roda porque a mudança foi comprovada e o diff é pequeno (`quando = "risco"`), a conversa registra "Revisão dispensada" e como pedir revisão sempre. Falha ao ler o diff também é registrada, em vez de a revisão sumir em silêncio.

## Front [0.13.0] — 2026-10-30

- O detalhe da Run mostra uma linha com tempo do agente, tempo das verificações, tokens de entrada e saída, custo e ciclos. Campo sem valor aparece como "não informado"; zero só quando o valor real é zero. Durações em segundos com uma casa decimal; tokens com separador de milhar.

## Back [0.13.1] — 2026-10-30

- Corrigido: o agente concluía que "o Bash está negado" e deixava de rodar testes e lint. Ele encadeava `ls … || bun install …; bun run test`, e como cada parte do comando precisa ser permitida (e `bun install`/`bunx` não são, de propósito: baixam e executam código de rede), tudo era negado. O prompt de escrita agora diz que as dependências já estão instaladas, manda rodar um comando simples por vez (`bun run test`, `bun run lint`, `bun run check`), proíbe instalação, `bunx` e encadeamento, explica que uma negação não desliga o Bash e pede editar arquivos pelas ferramentas de edição, não por redirecionamento. As permissões não mudaram.

## Front [0.12.0] — 2026-10-30

- **Chat** voltou como tela própria: runs à esquerda (em andamento no topo, anteriores recolhidas) e, à direita, a conversa ao vivo do agente — mensagens dele, ações agrupadas, logs técnicos recolhidos e o resultado da revisão — com o campo de nova tarefa embaixo (⌘↵ inicia). O Chat mostra quantas runs estão em andamento.
- A revisão aparece como cartão na conversa: quem revisou, o veredito e os achados com arquivo e linha, bloqueantes e sugestões.
- A configuração da run ficou num bloco recolhível que resume "quem implementa · quem revisa (quando)". Implementador e revisor escolhem agente (Claude Code ou Codex) e modelo; o Codex aceita o padrão da CLI ou um id digitado. **Quando revisar**: só com risco, sempre ou nunca.
- Iniciar uma implementação pelo painel da tarefa abre a conversa da run; o painel ganhou "Ver a conversa da última run". As runs saíram de baixo do quadro.
- Novos comandos `bun run app`, `app:build` e `app:install` para abrir, recompilar ou instalar o aplicativo.
- **BREAKING (IPC):** `startRun` envia `reviewerModel` e `reviewWhen`.

## Back [0.13.0] — 2026-10-30

- O Codex também pode **implementar** (`--sandbox workspace-write`, só o worktree, sem rede) e revisar; a escolha de agente e modelo vale para cada papel.
- `start_run` recebe `reviewer`, `reviewerModel` e `reviewWhen` (`auto`, `always`, `never`); `Run` ganha `reviewerModel` e `reviewWhen`; migração 10 (`runs.reviewer_model`, `runs.review_when`). **BREAKING:** os três argumentos são obrigatórios no IPC; o front 0.12.0 já os envia. `continue_run` mantém a escolha da run.
- "Sempre" revisa mesmo quando a Ação `revisar` diz `quando = "risco"`; "nunca" desliga a revisão sem editar a Ação. O evento `review` informa quem revisou.
- Modelo do Codex digitado é validado por caracteres de id (nunca vira opção da linha de comando).

## Front [0.11.0] — 2026-10-30

- Novo seletor de revisor (Claude Code, um modelo acima, ou Codex) ao lado de Agente e Modelo, na nova tarefa e no painel da tarefa. A escolha fica salva neste dispositivo.
- O Codex continua indisponível como agente que implementa; agora a lista diz "só revisa, por enquanto".

## Back [0.12.0] — 2026-10-30

- O Codex pode revisar uma run de escrita: novo adaptador `run/codex.rs` (`codex exec --json --sandbox read-only --ephemeral`, prompt por stdin, saída estruturada por `--output-schema`), usado só pelo revisor. O filtro de achados fora do diff e o ciclo de correção são os mesmos do revisor Claude.
- `start_run` recebe `reviewer` (`claude` ou `codex`). **BREAKING:** o argumento é obrigatório no IPC; o front 0.11.0 já o envia. `Run` ganha `reviewer`; migração 9 adiciona `runs.reviewer`. `continue_run` mantém o revisor da run.

## Front [0.10.0] — 2026-10-30

- Interface mais enxuta: removidos o texto de apresentação, o banner de início, o painel de contexto, o rodapé de status e a seção de conversas. A sidebar ficou com **Nova tarefa**, **Tarefas** e **Conexões**.
- A tarefa selecionada abre num painel lateral ao lado do quadro, com botão de fechar, em vez de aparecer abaixo das raias. Em janelas estreitas o painel desce para baixo do quadro.
- Cabeçalho da página com workspace e pasta; atalhos recolhidos em **Atalhos**; confirmações aparecem como toast sobreposto.
- **Nova tarefa** é só o campo de texto, o workspace, o agente e o botão; as sugestões de texto saíram da tela.
- **Conexões** virou uma lista com uma linha por CLI, sem textos explicativos.
- Atalhos: ⌘1 Tarefas, ⌘2 Nova tarefa, ⌘3 Conexões (antes, ⌘2 abria Conexões).

## Back [0.11.1] — 2026-10-30

- Corrigido: uma run do agente era bloqueada quando o comando composto usava `grep`, `sort`, `uniq`, `tr`, `stat`, `file`, `which`, `true` ou `false`, porque a lista de permissões não os tinha e cada parte do comando precisa ser permitida. Esses utilitários de leitura agora são permitidos; `sed`, `awk` e `xargs` continuam fora da lista, porque escrevem ou executam.

## Back [0.11.0] — 2026-10-30

- Ações de escrita aceitam `exigir_prova = true`: quando a contraprova termina em "nenhum teste prova a mudança" ou "nenhum teste alterado", a run ganha um ciclo de correção pedindo um teste que falhe sem a mudança (`trigger = proof`), uma vez por run e dentro do limite de correções. `exigir_prova` com leitura ou com `contraprova = false` é recusado.

## Front [0.9.0] — 2026-10-30

- **Mais ações** mostra a origem de cada Ação (embutida, sua ou do repositório) e o acesso. Ação do repositório nova ou alterada aparece desabilitada, com **Confirmar Ação do repositório**.
- Formulário **Nova Ação** (nome, id sugerido, descrição, acesso, modelo opcional e pedido com botões de variável): cria a Ação sem escrever TOML e ela aparece no menu na hora.

## Back [0.10.0] — 2026-10-30

- Ações versionadas no repositório (`.harnessy/actions/*.toml`): por id, vencem as do usuário e as embutidas, mas só rodam depois que o usuário confirma o sha256 do arquivo neste computador (`action_approvals`, migração 8). Alterar o arquivo volta a pedir confirmação. Novo comando `approve_action(workspaceId, id)`; `list_actions` ganha `pendente` e a origem `repositorio`.
- Novo comando `create_action(input)`: monta o TOML a partir do formulário, valida com o mesmo parser dos arquivos (variáveis, id, modelo) e grava em `<dados do app>/actions/<id>.toml`, sem sobrescrever nem reutilizar id de Ação embutida.
- Dependência nova: `sha2`.

## Front [0.8.0] — 2026-10-30

- Run mostra o veredito do revisor independente ("aprovou", "pediu mudanças" ou "não respondeu no formato esperado"), com a contagem de achados descartados por estarem fora do diff.
- "Dividir em passos" aparece em **Mais ações**. A run termina com os passos propostos, editáveis e removíveis; **Criar N tarefa(s)** cria as tarefas encadeadas em A fazer.
- Tarefa que é passo de uma cadeia oferece **Continuar das alterações de “<passo anterior>”** (marcado por padrão quando o passo anterior tem run concluída). Ações sobre run (revisor) não aparecem como botão.

## Back [0.9.0] — 2026-10-30

- Revisor independente (Ação embutida `revisar`, leitura, `alvo = "run"`, `saida = "revisao"`, `quando = "risco"`): roda só com run verde e, por padrão, só com sinal de risco (contraprova sem prova ou sem testes, ou diff acima de 300 linhas). Recebe tarefa, critérios, checks, contraprova e diff, nunca a sessão do implementador; usa um modelo acima do implementador. Achados fora das linhas alteradas do diff são descartados; `request_changes` com achado que bloqueia vira falha estruturada e um ciclo de correção (`trigger = review`), dentro do limite de correções. Revisão roda uma vez por run.
- Ações ganham `alvo` (`tarefa` ou `run`), `saida` (`texto`, `revisao`, `passos`) e `quando` (`risco`, `sempre`, `manual`), com as variáveis `{{diff}}`, `{{checks}}` e `{{contraprova}}` só em Ação sobre Run. Uma Ação do usuário com id `revisar` substitui a embutida; `quando = "manual"` desliga a revisão automática.
- Ação embutida `dividir-em-passos` (leitura, `saida = "passos"`) devolve até 5 passos por `--json-schema`; o evento `steps` da run traz a proposta. Novo comando `create_steps(workspaceId, parentId, steps)` cria as tarefas encadeadas (`parent_id`, `after_id`), tudo ou nada.
- `start_run` aceita `continueFrom` (opcional): a run parte da branch de uma run concluída do mesmo workspace; contraprova e revisor comparam com essa base. `runs.base_run_id` registra a origem.
- `list_runs` ganha `review`, `reviewDiscarded` e `baseRunId`; `list_tasks` ganha `parentId` e `afterId`; `list_actions` ganha `alvo` e `saida`.
- Migrações 6 e 7: `runs.review`, `runs.review_discarded`, `runs.base_run_id`, `tasks.parent_id`, `tasks.after_id`.

## Front [0.7.0] — 2026-10-30

- Run que falhou e manteve a pasta de trabalho oferece **Tentar com `<próximo modelo>`**: um ciclo com o modelo seguinte da lista, a partir das alterações já feitas. O botão some quando o último ciclo já usou o modelo mais forte.

## Back [0.8.0] — 2026-10-30

- Ciclos de correção retomam a sessão do agente (`--resume`) e enviam só as falhas. Se a retomada for recusada antes de qualquer evento, a run recomeça com o contexto completo, mostra "Sessão não retomada; contexto reenviado" e o ciclo não conta duas vezes. `run_cycles.resumed` registra a escolha.
- Novo comando `continue_run(runId)`: reabre uma run `failed` com worktree preservado e roda um ciclo com o próximo modelo da lista (`haiku → sonnet → opus → fable`), relativo ao modelo do último ciclo. O ciclo recebe a tarefa, as falhas atuais e o `git diff --stat`, com `trigger = bigger_model`. `list_runs` ganha `effort`, `lastModel` e `nextModel`.
- `{{arquivos_provaveis}}` passa a ser calculado: termos do título e dos critérios casados com nomes de arquivo (`git ls-files`) e conteúdo (`git grep`), até 10 caminhos, sem conteúdo e sem modelo.
- Comandos negados pelo Harnessy ficam lembrados por workspace (`workspace_denials`); o prompt das próximas runs ganha a regra "Não tente: …" com até 20 comandos.
- Ações aceitam `esforco` (`low`…`max`, repassado como `--effort`) e `max_turns` (até o limite de 25). O argumento `--exclude-dynamic-system-prompt-sections` mantém o prompt de sistema igual entre worktrees, para aproveitar o cache.
- Migração 5: `runs.effort`, tabela `workspace_denials` e `run_cycles` reconstruída com `resumed` e os gatilhos `diff_guard`, `proof`, `review` e `bigger_model` (dados existentes copiados).
- Teste ignorado por padrão (`instructions_are_loaded_by_the_cli`) responde se a CLI carrega o `CLAUDE.md` com os argumentos do Harnessy.

## Front [0.6.0] — 2026-10-30

- Cada run concluída mostra o veredito da contraprova, com ícone e texto: "Comprovado por testes", "Nenhum teste prova a mudança", "Nenhum teste alterado", "Contraprova não executada" ou "Contraprova inconclusiva". Sem contraprova, a linha não aparece.

## Back [0.7.0] — 2026-10-30

- Contraprova do teste (SPEC §5.2): depois do commit, os testes novos ou alterados da run são rodados sobre o código da base. O veredito (`proved`, `not_proved`, `no_tests`, `skipped` ou `inconclusive`) é aviso e não muda o estado da run. Eventos `status` (frase) e `proof` (JSON com veredito, testes e falhas) registram o resultado.
- Typecheck por padrão (SPEC §4.2): o script `typecheck` (ou `check`) do `package.json`, `cargo check` quando há `Cargo.toml` e `uv run mypy`/`pyright` quando o `pyproject.toml` configura um deles entram como checks obrigatórios, antes dos testes. `cargo test` entra quando o `package.json` não tem `test`.
- Ação com `contraprova = false` pula a etapa; o campo já existia no formato TOML e agora tem efeito.
- Migração 4: `runs.proof` (anulável). `list_runs` devolve `proof` em camelCase. Campo novo e opcional; quem não lê o campo não é afetado.
- Interno: o contrato `Sink` ganhou `proof`; o worktree só é removido quando a contraprova não falhou por infraestrutura.

## Front [0.5.0] — 2026-10-30

- Tarefa ganha o menu **Mais ações**, com nome, descrição e nível de acesso de cada Ação. Ação inválida aparece desabilitada, com o motivo em texto.
- O botão **Implementar** passa a ser a Ação `implementar`; a interface envia o id da Ação ao iniciar a run e ao mostrar a prévia do prompt.

## Back [0.6.0] — 2026-10-30

- **BREAKING:** `start_run` e `preview_prompt` recebem o parâmetro `action` (id da Ação). Novo comando `list_actions(workspaceId?)`. Runs antigas continuam com `action = "implement"`; não há migração de dados.
- Ações declarativas em TOML: `implementar` (escrita) e `explicar` (leitura) embutidas; Ações do usuário em `<dados do app>/actions/*.toml`, com precedência por id sobre as embutidas. Arquivo inválido aparece na lista com o erro e a linha, sem derrubar as demais.
- Prompt da run em duas partes: início fixo do núcleo, com as regras de segurança, e o pedido da Ação no final, que não consegue removê-las. Saiu a instrução de rodar as verificações do `AGENTS.md`: o núcleo roda as verificações ao final, e o agente pode rodá-las antes se quiser conferir.
- Ação de leitura roda na pasta do workspace só com `Read`, `Glob` e `Grep`, sem worktree, checks nem correção, e não move o card.
- Falhas de verificação passam a ser estruturadas por runner (Vitest/Jest, `cargo test`, `cargo check`/`clippy`, `tsc`, pytest, `go test`), com arquivo, linha, teste, esperado, obtido e trecho. O prompt de correção traz uma seção por falha (até 5 em detalhe, total de 8 KB); saída não reconhecida mantém o trecho final como antes. Cada ciclo grava as falhas como evento `failures` da run.

## Back [0.5.0] — 2026-10-30

- **BREAKING:** `list_runs` devolve o ledger de cada run: `model`, `cycles`, `turns`, `inputTokens`, `outputTokens`, `cacheReadTokens`, `cacheWriteTokens`, `costUsd`, `agentMs` e `checksMs`. Todos são `null` quando desconhecidos; um total só existe se todos os ciclos informaram o campo.
- **BREAKING:** migração 3 acrescenta esses campos a `runs` e cria `run_cycles` (uma linha por invocação do agente, com `trigger` `initial` ou `check_failed`). Bancos locais de versões anteriores migram sem perda.
- O uso do Claude Code (tokens, custo, turnos) e o `session_id` são lidos do stream-json por ciclo. Custo ausente vira `null` e a linha do log mostra "custo não informado", em vez de zero.
- Interno: o pipeline de run depende do contrato `Runtime` (`run/runtime.rs`), e o Claude Code é um adaptador (`run/claude.rs`). Comportamento do agente sem mudança.

## Front [0.4.1] — 2026-10-30

- Tipo `Run` do núcleo ganha os campos do ledger (modelo, ciclos, tokens, custo e tempos). Sem tela nova.

## Back [0.4.1] — 2026-10-30

- Corrigido: o app, aberto pelo Finder, não herdava o `PATH` do shell e achava um Node antigo (`/usr/local/bin`), o que quebrava o `vitest` nas verificações da run e levava o agente a alterar o `package.json` para contornar o ambiente. O `PATH` do shell de login do usuário (nvm, asdf, bun) é lido uma vez na abertura, em segundo plano, e passa a vir primeiro na busca de binários, nas verificações e no ambiente do agente. Se a leitura falhar ou passar de 8 s, vale o `PATH` comum.
- Corrigido: um comando composto do agente (`cd … && cat …; ls; git status`) era negado inteiro porque `cd`, `cat`, `ls` e `find` não estavam permitidos, e o agente concluía que não tinha Bash e deixava de rodar os checks. Agora a leitura e a navegação (`cd`, `ls`, `cat`, `head`, `tail`, `wc`, `echo`, `pwd`, `find`, `git branch/show/ls-files/rev-parse`, versões de `node`/`bun`) são permitidas; `find -exec/-delete` e caminhos sensíveis (`.ssh`, `.aws`, `.config/gh`, credenciais do Claude, `.env*`) seguem negados por texto.
- O motivo do bloqueio por ações negadas mostra cada comando em uma linha de até 100 bytes; o comando completo continua no log da run.

## Front [0.4.0] — 2026-10-30

- Chat: o **Iniciar** funciona e não exige o Codex conectado. Cria a tarefa a partir do rascunho (primeira linha = título), limpa o rascunho, começa a run e leva para Tarefas.
- Seletor de **Agente** e **Modelo** no chat e no detalhe da tarefa. Padrão: Claude Code com Sonnet; modelos Fable, Opus, Sonnet e Haiku; Codex listado e desabilitado, com o motivo. A escolha é lembrada neste computador.
- Workspaces e a seleção passam a ser compartilhados entre o chat e as tarefas.

## Back [0.4.0] — 2026-10-30

- **BREAKING:** `start_run` recebe `runtime` e `model`. Só `claude` executa; outro agente devolve erro claro. O modelo é validado contra a lista `run::guard::MODELS` (aliases `fable`, `opus`, `sonnet`, `haiku`), então nada fora dela chega ao processo.
- O log da run informa o modelo usado em cada tentativa.

## Front [0.3.0] — 2026-10-30

- Selo **Bloqueada** no card (com o motivo) quando a última run da tarefa falhou, terminou sem mudar nada ou teve ações negadas; faixa no detalhe com **Marcar como resolvido**. Uma run nova supera o bloqueio anterior.
- O histórico mostra só você e o Harnessy; o resumo final do agente aparece como nota dos movimentos feitos pelo Harnessy.

## Back [0.3.0] — 2026-10-30

- **BREAKING:** removido o canal do agente (ADR 0002): `harnessy --cli mv|note`, o socket por run, o shim no `PATH`, as variáveis `HARNESSY_SOCKET`/`HARNESSY_TOKEN`, as permissões `Bash(harnessy …)` e as duas linhas do prompt. Gerir cards passa a custar zero token (ADR 0003): o card é movido pelo resultado da run e o resumo do agente vira a nota do histórico.
- **BREAKING:** `runs.blocked` na migração 2 (esquema editado no lugar); bancos locais antigos precisam ser apagados. `Run` ganha `blocked`; o ator `agent` deixa de existir; `Status` aceita só o nome exato.
- Bloqueio deduzido sem custo: run falhou, terminou sem mudar nada ou o `claude` negou ações (`permission_denials` viram eventos `denied` no log da run). Novo comando `resolve_block`.
- `no_changes` passa a guardar o resumo do agente, que explica por que nada mudou.

## Back [0.2.1] — 2026-10-30

- Corrigido: a janela não tinha permissão para escutar eventos do núcleo, então `task-changed`, `run-state` e `run-event` não chegavam à interface. A capability `main` agora concede só `core:event:allow-listen` e `core:event:allow-unlisten`.
- Corrigido: ao reabrir o app, tarefas de runs interrompidas ficavam presas em Em andamento. A abertura agora interrompe as runs ativas e devolve os cards a A fazer, com nota no histórico.
- Corrigido: o "bloqueado" do agente (`harnessy mv todo`) era sobrescrito quando a run terminava bem. A regra run → card ficou em `flow::sync::reconcile`: a falha da run vence a palavra do agente, e um movimento do agente para outro estado vence uma run concluída.
- Testes novos: regra de sincronização, recuperação na abertura, comandos do agente e o canal ponta a ponta (cliente → socket → banco). Teste `#[ignore]` com o Claude Code real (`cargo test the_real_agent -- --ignored`).

## Front [0.2.0] — 2026-10-30

- **BREAKING:** removidas a conexão com o GitHub Project, a fonte GitHub, o painel "Preparar ambiente" e a criação de Issues. As tarefas passam a ser só locais.
- A tela de tarefas virou um quadro de quatro raias (A fazer, Em andamento, Revisão, Concluído) com as cores do produto. Criar é digitar o título e teclar Enter (`n` foca o campo).
- Mover um card é imediato: botão de avançar no card, `]`/`[` e `1`–`4` no teclado, ou arrastar. Aviso com **Desfazer**; se o núcleo recusar, o card volta.
- Detalhe da tarefa com estado em um clique, edição, prompt do agente e histórico de movimentos (você, Harnessy ou agente), atualizado ao vivo por `task-changed`.
- Abas Tarefas locais e Board unificadas em **Tarefas**.
- Removidos o componente `Spinner`, o script `scripts/setup-github-project.sh` e o CSS do antigo board estático (`board-*`, `flow-strip`, `empty-art` e afins).

## Back [0.2.0] — 2026-10-30

- **BREAKING:** removidos os comandos `set_workspace_project`, `list_source_tasks`, `get_board`, `move_card`, `create_task`, `mark_ready`, `inspect_setup`, `apply_setup`, `start_scope_auth`, `cancel_scope_auth` e `open_device_page`, os módulos `source/` e `setup/` e a integração com `gh project`. `start_run` e `preview_prompt` recebem `taskId`.
- **BREAKING:** esquema editado no lugar (exceção autorizada enquanto não há dados a preservar; regra 13): `tasks.status` e `task_events` na migração 1, `runs.task_id` no lugar de `source`/`external_id`, migração de `project_number` removida (`user_version` 2). Bancos locais antigos precisam ser apagados.
- Tarefas ganham estado (`todo`, `doing`, `review`, `done`) e histórico de transições gravado na mesma transação. Novos comandos `move_task` e `task_history`; evento `task-changed`. Mover não altera `revision`.
- O estado da run move a tarefa (Em andamento, Revisão, A fazer) sem reabrir uma tarefa Concluída.
- Removida a variante de erro `Source`, sem uso depois da saída do módulo `source/`.
- Canal do agente (ADR 0002): socket Unix por run com token, shim `harnessy` no `PATH` e `harnessy --cli mv|note`. O agente só altera a tarefa da própria run; a resposta é `ok` ou `err <código>` e o prompt ganha duas linhas.

## Back [0.1.0] — 2026-10-30

- Loop de verificação: o núcleo roda os checks do projeto depois de cada tentativa do agente e envia até três correções com a saída das falhas. Uma run só termina como concluída com todos os checks verdes.
- Verificações detectadas a partir do script `test` de `package.json`.
- Experimento `cargo test experiment -- --ignored` para comparar um modelo com e sem o loop, com fixtures em `experiments/loop/`.

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
- Núcleo Rust reorganizado por domínio com erro único e IPC fino; regras em `.claude/rules/rust-architecture.md` e mapa em `docs/architecture-rust.md`. Sem mudança de comportamento.
- Botão para o Claude Code implementar uma tarefa em um worktree próprio, com log em tempo real, cancelamento e commit na branch `harnessy/run-<id>-<slug>`. Execução sem sandbox, restrita a workspaces confiáveis (ADR 0001); push e PR ainda são manuais.
