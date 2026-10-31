# Harnessy

Aplicativo desktop local que coordena agentes de código (Claude Code, Codex) sobre tarefas de um projeto, da escolha da tarefa ao PR.

## Language

**Workspace**:
Pasta local de um projeto, com repositório, ramo base e comandos de verificação configurados.
_Avoid_: Projeto, repo

**Tarefa**:
Item de trabalho guardado localmente, com título, descrição, critérios de aceitação e Estado. É a entrada de uma Run. Aparece como card no quadro.
_Avoid_: Task, ticket, issue

**Estado**:
Raia da Tarefa no quadro: `todo` (A fazer), `doing` (Em andamento), `review` (Revisão) ou `done` (Concluído). Qualquer Estado pode ir para qualquer outro.
_Avoid_: Status, coluna

**Histórico**:
Lista de eventos de uma Tarefa (quem moveu, de qual Estado para qual, nota opcional). Quem move é o usuário ou o Harnessy (pelo resultado da Run); o agente não move cards.
_Avoid_: Log (o log é da Run), auditoria

**Ação**:
Definição reutilizável de um passo automatizado (prompt, papel, entradas, acesso e permissões) que aparece como botão.
_Avoid_: Workflow, skill, macro, comando

**Run**:
Uma execução de uma Ação sobre uma Tarefa, com estado, eventos e resultado registrados.
_Avoid_: Execução, job, tarefa em andamento

**Bloqueio**:
Motivo objetivo de uma Run ter ficado presa: falhou, terminou sem mudar nada ou o agente teve ações negadas. É deduzido pelo Harnessy, sem custo de tokens, aparece como selo no card e é dado por resolvido pelo usuário.
_Avoid_: Impedimento, erro (um Bloqueio nem sempre é erro)

**Contraprova**:
Prova de que os testes da Run pegam a mudança: roda os testes novos ou alterados sobre o código da base. Testes que falham sem a mudança provam (`proved`); testes que já passam não provam (`not_proved`). É aviso, não bloqueio, e não usa tokens.
_Avoid_: Revisão, validação (a Contraprova é mecânica; o revisor é outro papel)

**Papel**:
Função de um agente no fluxo (refinador, implementador, revisor, tester), com runtime fixo em `src/lib/roles.ts`.
_Avoid_: Agente (o agente é quem cumpre o Papel)

**Worktree**:
Cópia de trabalho git isolada, criada para uma Run de escrita, em ramo próprio `harnessy/<run-id>-<slug>`.
_Avoid_: Sandbox (um worktree não isola o processo)

**Isolamento**:
Nível de contenção do processo do agente em um Workspace: `none` (só o worktree) ou `sandbox` (reservado, ainda não implementado).
_Avoid_: Permissões

**Perfil**:
Descrição confirmada de como instalar e verificar um Workspace (comandos de instalação, Checks e caminhos protegidos), em `.harnessy/workspace.toml`.
_Avoid_: Config, setup

**Check**:
Comando de verificação do Perfil executado pelo núcleo, com resultado registrado como evidência de uma Run.
_Avoid_: Teste (um Check pode ser lint ou typecheck), validação

**Ciclo**:
Uma rodada de correção dentro de uma Run, iniciada quando um Check obrigatório falha; limitada a três por padrão.
_Avoid_: Retry, tentativa

**Runtime**:
CLI de agente que executa um ciclo (hoje Claude Code), atrás do contrato `run::runtime::Runtime`. Só o adaptador do runtime conhece a CLI.
_Avoid_: Provider, motor

**Revisor**:
Ação de leitura sobre o resultado de uma Run verde, com modelo acima do implementador e sem a conversa dele. Opina; achados só viram correção depois do filtro contra as linhas alteradas do diff.
_Avoid_: Reviewer, juiz

**Refinamento**:
Run de leitura da Ação "Refinar": devolve perguntas com resposta recomendada e um rascunho de descrição e critérios (evento `refinement`). Só vira texto da Tarefa quando a pessoa aplica.
_Avoid_: Spec, planejamento

**Passo**:
Tarefa criada por "Dividir em passos", ligada à tarefa de origem e ao passo anterior; a Run de um passo pode partir da branch do anterior.
_Avoid_: Subtarefa, etapa

**Entrega**:
Caminho de uma Tarefa até o projeto principal, a partir da branch de uma Run: só local, PR em draft, checks, revisão aprovada, merge. Uma Run publicada tem no máximo um PR; publicar de novo adota o que existe.
_Avoid_: Deploy (deploy publica uma versão em ambiente; Entrega termina no merge)

**Estágio**:
Ponto da Entrega em que a Tarefa está (`local`, `draft`, `checks-failing`, `ready`, `merged`…), deduzido de fatos (Run, PR, checks, revisão, SHA), nunca gravado. Define o Próximo passo.
_Avoid_: Estado (Estado é a raia do quadro)

**Próximo passo**:
A única ação principal oferecida numa Tarefa, deduzida do Estágio. As demais ficam em "Mais ações".
_Avoid_: Fluxo, workflow

**Ambiente de teste**:
O projeto ligado na máquina do usuário, no worktree de uma Run, para ver a mudança funcionando. É o primeiro alvo de ambiente; um deploy em QA entra no mesmo lugar. Um por vez.
_Avoid_: Preview (termo do código), sandbox

**Perfil do workspace**:
Arquivo `.harnessy/workspace.toml` do repositório com o comando que sobe o projeto. Só roda depois de o usuário confirmar o texto exato (sha256).
_Avoid_: Configuração

**Atenção**:
O que espera uma decisão do humano (bloqueio, checks falhando, pronto para merge…). Alimenta o filtro "Precisa de você", o contador da bandeja e as notificações. Concluída, em andamento com agente e "checks rodando" nunca pedem atenção.
_Avoid_: Pendência, alerta

**Harness do núcleo / do projeto**:
Duas camadas de regras. O do núcleo (prefixo do prompt, Ações embutidas, `json_schema`, permissões geradas em Rust) manda e vive no executável. O do projeto (AGENTS.md, CLAUDE.md, `.claude/rules`, ADRs) informa convenções e entra como dado delimitado, lido do commit base. Ver [ADR 0004](docs/adr/0004-duas-camadas-de-harness.md).
_Avoid_: Instruções, system prompt

**Revisão (staff)**:
Papel do revisor: avalia simplicidade, padrões do projeto, testabilidade e manutenção sobre o diff. Bugs e casos de borda são do QA, papel separado ainda não implementado.
_Avoid_: Code review genérico, teste

**Modo (de automação)**:
Quanto o humano delega numa etapa do ciclo (iniciar, revisão, QA, ambiente, publicar, merge): `manual` (eu conduzo), `assisted` (o Harnessy prepara e pede o OK) ou `auto` (executa se as travas passam). O merge nunca é `auto`. É escolha pessoal e local; o repositório só pode apertar as travas.
_Avoid_: Permissão, nível
