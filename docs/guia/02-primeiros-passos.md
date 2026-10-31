# 2. Primeiros passos

[← Guia](../user-guide.md) · [← Anterior](01-o-que-e-o-harnessy.md) · [Próximo: Tarefas e o quadro →](03-tarefas-e-quadro.md)

## 1. Abrir o aplicativo

Se alguém já gerou o aplicativo para você, abra o `Harnessy.app` pelo Finder. Se você ainda precisa instalar ou gerar, siga a [instalação](../getting-started.md) (a parte técnica fica lá).

Ao fechar a janela, o Harnessy **continua rodando** e fica na barra de menus do Mac, no ícone **H**. Clique nele para **Abrir Harnessy** ou **Encerrar Harnessy**. Isso é normal e é o que permite avisar você mesmo com a janela fechada.

## 2. Conectar as ferramentas

Abra **Conexões** (`⌘3`). Cada cartão mostra uma ferramenta e seu estado:

| Estado | O que significa | O que fazer |
|---|---|---|
| **Conectado** | Pronta para usar | Nada |
| **Sessão expirada** | Instalada, mas sem login válido | Rode o comando mostrado no cartão no Terminal e clique em **Verificar novamente** |
| **Indisponível** | Não foi encontrada neste Mac | Instale com o comando sugerido no cartão |
| **Não verificado** | Não respondeu a tempo | Rode o comando de status no Terminal para ver o erro |

O que cada uma faz:

- **Claude Code (`claude`)**: é o agente que escreve o código. **Obrigatório** para implementar.
- **GitHub (`gh`)**: usado para enviar a mudança e abrir o PR. Necessário só para publicar.
- **Codex (`codex`)**: opcional, pode ser usado como revisor.

O Harnessy **nunca pede sua senha nem token**. O login é feito no Terminal, nas próprias ferramentas.

## 3. Criar um workspace

Um **workspace** é a pasta do seu projeto no Mac.

1. Em **Tarefas**, clique para criar um workspace.
2. Dê um nome e informe o **caminho completo** da pasta, que começa com `/` (por exemplo, `/Users/seu-nome/projetos/meu-app`). Para ver o caminho, arraste a pasta do Finder para o Terminal.
3. Pronto: o workspace aparece na barra lateral. Você pode ter vários e trocar clicando neles.

A pasta precisa ser um repositório **git** para o agente conseguir trabalhar; se não for, o Harnessy avisa na hora de implementar.

> Use o Harnessy só em projetos em que você confia. O agente trabalha em uma cópia separada, mas **não há isolamento forte**; veja [Segurança e privacidade](09-seguranca-e-privacidade.md).

## 4. Sua primeira tarefa

1. Na coluna **A fazer**, digite o título (por exemplo, "Corrigir o texto do botão de login") e tecle `Enter`.
2. Clique em **+ Descrição e critérios** para explicar melhor. Só o título é obrigatório, mas **quanto mais claro o que é "pronto", melhor o resultado**.
3. Selecione a tarefa. No painel ao lado aparece o **Próximo passo**: **Implementar**. Clique ou tecle `A`.
4. O card vai para **Em andamento** e o agente trabalha. Você pode usar outros aplicativos: o Harnessy avisa quando precisar de você.

Continue em [Tarefas e o quadro](03-tarefas-e-quadro.md).

## Fechar, reabrir e atualizar

| Ação | Resultado |
|---|---|
| Fechar a janela (botão vermelho) | Esconde a janela; o app segue ativo |
| **H → Abrir Harnessy** ou clicar no app no Dock | Mostra a janela |
| **H → Encerrar Harnessy** | Encerra o app (e para ambientes de teste que estiverem no ar) |

Não há atualização automática: para uma versão nova, gere o app de novo pelo [guia de instalação](../getting-started.md) e substitua a cópia antiga.

## Se algo não funcionar

Veja a [solução de problemas](../troubleshooting.md).
