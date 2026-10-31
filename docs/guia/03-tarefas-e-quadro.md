# 3. Tarefas e o quadro

[← Guia](../user-guide.md) · [← Anterior](02-primeiros-passos.md) · [Próximo: O agente e as runs →](04-agentes-e-runs.md)

## Para que serve

O quadro é o seu painel. Cada **tarefa** é um card, e a coluna em que ele está diz em que pé o trabalho está.

| Coluna | Significa |
|---|---|
| **A fazer** | Ainda não começou |
| **Em andamento** | O agente está trabalhando |
| **Revisão** | O agente terminou e **espera você** (ler, testar, publicar) |
| **Concluído** | Terminou |

O Harnessy **move os cards sozinho** conforme o resultado do agente. Você também pode mover à mão, a qualquer momento.

## Criar uma tarefa

1. No campo **Título** da coluna **A fazer**, digite e tecle `Enter` (ou clique em **Criar**). O atalho `n` leva o cursor até ele.
2. Para detalhar, clique em **+ Descrição e critérios**:
   - **Descrição**: o que você quer, com o contexto que a pessoa que vai fazer precisaria saber.
   - **Critérios de aceite**: uma lista do que precisa ser verdade para estar pronto (por exemplo, "o botão mostra 'Entrar'").
3. `Ctrl+Enter` ou `⌘+Enter` dentro dos campos de texto cria a tarefa.

**Dica:** critérios claros são o que mais melhora o resultado do agente e a sua revisão depois.

## Mexer nos cards

| Quero… | Como |
|---|---|
| Abrir os detalhes | Clique no card (ele fica selecionado e o detalhe aparece embaixo) |
| Editar título, descrição e critérios | Abra o card e edite; ou use o ícone de lápis |
| Mover de coluna | A seta no card, os botões de **Estado da tarefa** no detalhe, as teclas `]` (avança) e `[` (volta), os números `1` a `4`, ou arrastar |
| Desfazer um movimento | **Desfazer** no aviso que aparece por alguns segundos |
| Marcar como concluída ou reabrir | Em **Mais ações**: **Concluir tarefa** / **Reabrir tarefa** |
| Excluir | Ícone de lixeira ou **Excluir tarefa**; o Harnessy pede confirmação. As branches no git ficam |
| Ver o que aconteceu | **Histórico** no detalhe: cada movimento, quem fez (você ou o Harnessy) e o motivo |

Toda a mudança de coluna fica no histórico, então nada some sem registro.

## O que os selos nos cards dizem

| Selo | Significa |
|---|---|
| **agente trabalhando** | Há uma run em andamento |
| **Bloqueada** | A última run ficou presa (falhou, não mudou nada ou teve ações negadas). Passe o mouse para ver o motivo |
| **Só local**, **PR em draft**, **Checks falhando**, **Pronto para merge**… | O estágio da entrega. Veja [Publicar e fazer merge](07-publicar-e-merge.md) |
| **No ar** | O ambiente de teste da tarefa está rodando. Veja [Testar localmente](06-testar-localmente.md) |

Uma tarefa **Bloqueada** não é necessariamente um erro seu: clique nela, leia o motivo, e escolha entre pedir um ajuste, tentar de novo ou **Dispensar aviso** se você já tratou o problema por fora.

## Separar o que precisa de você

Acima do quadro há três filtros: **Todas**, **Precisa de você** e **Com os agentes**, cada um com a contagem. Use **Precisa de você** para ver só o que está esperando uma decisão sua. Veja [Atenção e notificações](08-atencao-e-notificacoes.md).

## Dividir uma tarefa grande

Uma tarefa grande demais costuma dar resultados piores. No detalhe da tarefa (embaixo do quadro), abra **Mais ações** e escolha **Dividir em passos**: o agente propõe tarefas menores, encadeadas, que continuam uma da outra. Você revisa a proposta antes de criar os passos.

## Refinar uma tarefa antes de implementar

Quando a tarefa ainda está vaga, use **Refinar** nos comandos rápidos. O agente só lê o projeto: ele faz até 6 perguntas, cada uma com a resposta que recomenda, e propõe uma descrição e critérios de aceitação. O resultado aparece no Chat, em **Refinamento proposto**:

1. Responda às perguntas (a recomendação já vem preenchida; deixe como está para aceitá-la) e edite o rascunho.
2. **Refinar de novo** envia suas respostas e gera outra rodada, até não restarem perguntas.
3. **Aplicar à tarefa** substitui a descrição e os critérios atuais pelo rascunho e registra, na descrição, uma seção "Decisões" com as suas respostas.

As perguntas são fechadas (sim/não ou opções) e vêm com um exemplo; os critérios são cenários que dá para conferir na tela ou em um teste. Quando aparece **Pronta para implementar**, nenhuma pergunta em aberto mudaria o rascunho. Cada conversa permite até 2 rodadas. Se você editou a tarefa depois que o rascunho apareceu, **Aplicar à tarefa** recusa em vez de sobrescrever.

Refinar não move o card nem altera o projeto. O botão fica indisponível com edições não salvas, com uma run em andamento e com a tarefa concluída.

## Atalhos de teclado

| Ação | Atalho |
|---|---|
| Ir para Tarefas / Chat / Conexões | `⌘1` / `⌘2` / `⌘3` |
| Nova tarefa (cursor no título) | `n` |
| Executar o **Próximo passo** da tarefa selecionada | `A` |
| Mover o card focado | `]` avança, `[` volta, `1`–`4` vai direto à coluna |
| Percorrer controles | `Tab` / `Shift+Tab` |
| Ativar o botão com foco | `Enter` ou `Espaço` |
| Fechar um diálogo | `Esc` |

O aplicativo respeita a preferência **Reduzir movimento** do Mac e pode ser usado só com o teclado. O primeiro link ao apertar `Tab` permite pular a barra lateral.

Próximo: [O agente e as runs →](04-agentes-e-runs.md)
