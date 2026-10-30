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

**Passo**:
Tarefa criada por "Dividir em passos", ligada à tarefa de origem e ao passo anterior; a Run de um passo pode partir da branch do anterior.
_Avoid_: Subtarefa, etapa
