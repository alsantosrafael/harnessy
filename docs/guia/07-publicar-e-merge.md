# 7. Publicar e fazer merge

[← Guia](../user-guide.md) · [← Anterior](06-testar-localmente.md) · [Próximo: Atenção e notificações →](08-atencao-e-notificacoes.md)

## Para que serve

Depois que você leu e testou, a mudança precisa chegar ao projeto principal. No GitHub isso acontece em duas etapas: primeiro um **PR** (pedido para incluir a mudança, onde ela pode ser conferida por pessoas e por testes automáticos), depois o **merge** (incluir de fato). O Harnessy faz as duas, **só quando você manda**.

> Estas são as duas ações que **saem do seu Mac**. Por isso o agente nunca as faz: quem faz é o Harnessy, com o login do GitHub que você já fez no Terminal.

## Antes de começar

- **GitHub (`gh`)** precisa estar **Conectado** em **Conexões**.
- A tarefa precisa ter uma run concluída **com alterações**.

## Publicar o PR

1. Selecione a tarefa. Quando for a hora, o botão principal é **Publicar PR** (ou use **Mais ações**).
2. O Harnessy **confere o que vai enviar**: recusa se não há alterações e **recusa se o conjunto inclui arquivos que parecem conter segredos** (como `.env` ou chaves).
3. Envia a branch ao GitHub e abre o PR **como rascunho (draft)**. O texto do PR traz a tarefa, os critérios, o resumo da run, a contraprova, a revisão e a lista de arquivos, e termina com "Publicado pelo Harnessy · run #N".
4. Clicar de novo **não cria um segundo PR**: o botão vira **Atualizar PR** e só relê o estado.

O PR nasce como **draft** de propósito: ele avisa que ainda não está para valer.

## Os estágios da entrega

O selo no card diz onde a entrega está. Cada um tem uma próxima ação:

| Selo | Significa | Próximo passo |
|---|---|---|
| **Só local** | A branch só existe no seu Mac | **Rodar localmente**, depois **Publicar PR** |
| **PR em draft** / **PR aberto** | Publicado, sem revisão aprovada neste commit | **Revisar** |
| **Checks rodando** | Os testes automáticos do GitHub estão rodando | Esperar (ou **Atualizar PR**) |
| **Checks falhando** | Algum teste automático reprovou | **Pedir ajuste** |
| **Conflito** | A mudança colide com algo que mudou no projeto | **Pedir ajuste** |
| **Mudanças pedidas** | A revisão pediu mudanças e nada foi corrigido | **Pedir ajuste** |
| **Pronto para merge** | Sem falhas, sem conflito e revisão aprovada **neste commit** | **Fazer merge** |
| **Mergeado** | Incluído no projeto principal | — |
| **PR fechado** | Fechado sem incluir | **Publicar de novo** |

**Por que a aprovação é "neste commit":** se o código muda depois da revisão, a aprovação antiga perde a validade. O Harnessy não deixa você mesclar algo diferente do que foi aprovado.

O Harnessy relê os PRs abertos **quando você volta para a janela e a cada 90 segundos**, então os selos acompanham o GitHub.

## Fazer o merge

1. Quando o selo for **Pronto para merge**, o botão principal é **Fazer merge**.
2. Um diálogo explica o que vai acontecer: o PR será incluído em **um único commit** (*squash*) na branch base, e a tarefa vai para **Concluído**. Confirme.
3. O Harnessy **relê o PR** e só mescla o commit que você acabou de ver. Se algo mudou nesse meio-tempo, ele recusa.

O merge é **recusado** (com o motivo) se: o PR não está aberto, há checks reprovados ou ainda rodando, há conflito, ou a revisão pediu mudanças sem correção.

> O merge **não é desfeito pelo Harnessy**. Se precisar voltar atrás, use o GitHub (*revert*).

## Se algo der errado

| Sintoma | O que fazer |
|---|---|
| "GitHub CLI (gh) não encontrado" | Instale o `gh` e faça `gh auth login --web`; confira em **Conexões** |
| "Publicação recusada: o diff inclui…" | Um arquivo parecido com segredo entrou na mudança. Peça ao agente para removê-lo |
| Os checks nunca terminam | Abra o PR no GitHub (o endereço está no registro da run) e veja o que o CI está fazendo |
| O PR mostra mais mudanças do que a tarefa | A branch base do seu workspace não é a padrão do GitHub e carrega commits ainda não publicados. Publique-os antes ou aceite o ruído |
| Selo não atualiza | Volte à janela ou clique em **Atualizar PR** |

Próximo: [Atenção e notificações →](08-atencao-e-notificacoes.md)
