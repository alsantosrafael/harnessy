# 8. Atenção e notificações

[← Guia](../user-guide.md) · [← Anterior](07-publicar-e-merge.md) · [Próximo: Segurança e privacidade →](09-seguranca-e-privacidade.md)

## Para que serve

Agentes levam minutos para trabalhar, e **você não deveria ficar olhando**. O Harnessy separa o que **espera você** do que **está com os agentes** e te chama na hora certa, mesmo com a janela escondida.

## O que "precisa de você"

Uma tarefa pede sua atenção quando está esperando uma decisão sua:

| Situação | Por quê |
|---|---|
| **Bloqueada** | A run ficou presa e precisa de uma escolha sua |
| **Checks falhando** / **Conflito** | É preciso pedir um ajuste |
| **Mudanças pedidas** | A revisão pediu correções |
| **Pronto para merge** | Só falta a sua decisão |
| **PR em draft / PR aberto** | Falta revisar |
| **Só local** | Pronto para você testar e publicar |
| **PR fechado** | Sem merge; decida o que fazer |

Ficam **de fora**: tarefas concluídas, o agente trabalhando e **Checks rodando** (isso é com os agentes e o GitHub).

## No quadro

Acima do quadro:

- **Todas**, **Precisa de você** e **Com os agentes**: filtros com contagem. O de "Precisa de você" fica destacado quando há pendências.
- Uma frase ao lado ("2 tarefas pedem você") que um leitor de tela também anuncia quando muda.

## Na barra de menus

O ícone **H** ganha um número: **H 3** significa que três tarefas pedem você. Passe o mouse para ver a frase completa. Sem pendências, volta a ser só **H**.

## Notificações

Quando algo **novo** passa a precisar de você e a janela do Harnessy **não está em foco**, o macOS mostra uma notificação:

- uma tarefa: **"Precisa de você"**, com o número, o título e o motivo;
- várias: **"N tarefas pedem você"**, com a mais urgente.

Na primeira vez, o macOS pede permissão. Se você recusar, o Harnessy não insiste; você ainda vê o contador no **H** e o resumo ao voltar. Para mudar depois: **Ajustes do Sistema → Notificações → Harnessy**.

O que **já estava** pendente quando você abriu o app ou trocou de workspace **não** gera notificação: só as novidades.

## Ao voltar: "Enquanto você esteve fora"

Se coisas chegaram enquanto a janela estava sem foco, ao voltar aparece um aviso com a lista, **do mais urgente ao menos**. Clique no nome de uma tarefa para abri-la, **Ver o que precisa de mim** para filtrar, ou **Dispensar**.

## Se algo der errado

| Sintoma | O que fazer |
|---|---|
| Não chegou notificação | Veja **Ajustes do Sistema → Notificações**. Em modo de desenvolvimento, o macOS pode atribuir a notificação ao Terminal |
| O número no **H** não aparece | Ele só mostra pendências do workspace aberto. Confira se há workspace selecionado |
| Recebi aviso de algo que já tinha visto | Só novidades notificam; um motivo novo na mesma tarefa conta como novidade |

Próximo: [Segurança e privacidade →](09-seguranca-e-privacidade.md)
