# Changelog

O produto segue SemVer. Durante 0.x, incompatibilidades exigem uma versão minor e migração explícita.

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
