# Glossário

[← Guia](../user-guide.md) · [← Anterior](09-seguranca-e-privacidade.md)

Palavras que aparecem no Harnessy, explicadas sem jargão. Em ordem alfabética.

| Palavra | O que é |
|---|---|
| **Ação** | Um pedido reutilizável ao agente, como **Explicar** ou **Dividir em passos**. Você pode criar as suas em **Nova Ação** |
| **Agente** | O programa de IA que escreve o código. Hoje, o Claude Code |
| **Alterações da run** | A lista do que o agente mudou, arquivo por arquivo, para você ler |
| **Ambiente de teste** | O projeto ligado no seu Mac, com a mudança do agente, para você testar de verdade |
| **Atenção** | O que espera uma decisão sua. Aparece no filtro **Precisa de você**, no número do **H** e em notificações |
| **Bloqueada** | Selo de uma tarefa cuja última run ficou presa (falhou, não mudou nada ou teve ações negadas) |
| **Branch** | Uma versão paralela do projeto, onde a mudança é feita sem tocar na original. As do Harnessy se chamam `harnessy/run-…` |
| **Checks** | Testes automáticos que o GitHub roda num PR (também chamados de CI) |
| **Commit** | Um "ponto salvo" no histórico do projeto, com uma descrição |
| **Conexões** | A tela que mostra se Claude Code, GitHub e Codex estão prontos para uso |
| **Contraprova** | Verificação de que os testes novos realmente pegam a mudança (falham sem ela) |
| **Critérios de aceite** | A lista do que precisa ser verdade para a tarefa estar pronta |
| **Dispensar aviso** | Botão de uma tarefa **Bloqueada**: tira o aviso de bloqueio sem mudar a coluna da tarefa |
| **Draft** | Um PR "rascunho". Avisa que ainda não está para valer |
| **Entrega** | O caminho da mudança até o projeto principal: só local, PR, checks, revisão, merge |
| **Estágio** | O ponto da entrega em que a tarefa está (o selo no card) |
| **Git** | O programa que guarda o histórico do projeto. O Harnessy exige que a pasta do workspace seja um repositório git |
| **GitHub CLI (`gh`)** | O programa que o Harnessy usa para falar com o GitHub, com o seu login |
| **Harness** | Tudo que cerca o agente para ele trabalhar bem: instruções, regras, verificações. O **do Harnessy** é fixo e manda; o **do projeto** (como o `AGENTS.md`) informa as convenções |
| **Merge** | Incluir a mudança no projeto principal. É o passo final e é decisão sua |
| **Modelo** | O "tamanho do cérebro" do agente: maior resolve mais, custa mais e demora mais |
| **Perfil do workspace** | O arquivo `.harnessy/workspace.toml` com o comando que sobe o projeto |
| **Pronto para merge** | Selo: sem falhas, sem conflito e revisão aprovada neste commit |
| **PR (pull request)** | O pedido para incluir a mudança no projeto principal, no GitHub |
| **Próximo passo** | O botão principal da tarefa: a ação que o Harnessy considera a certa agora |
| **Quadro** | As quatro colunas com as suas tarefas |
| **Revisão** | A opinião de um segundo agente (um "staff engineer") sobre o código. Ajuda, mas não substitui você |
| **Revisor** | Quem faz a revisão: Claude Code ou Codex |
| **Run** | Uma tentativa do agente de fazer uma tarefa, com registro, tempo e custo |
| **Squash** | Juntar todos os commits do PR em um só no merge |
| **Tarefa** | Um pedido: título, descrição e critérios de aceite. Vira um card |
| **Token** | A unidade em que os modelos "leem e escrevem" (aproximadamente pedaços de palavras). Define parte do custo |
| **Workspace** | A pasta do seu projeto no Mac |
| **Worktree** | A cópia separada do projeto em que o agente trabalha, para o original não ser tocado |
