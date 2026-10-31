# 1. O que é o Harnessy

[← Guia](../user-guide.md) · [Próximo: Primeiros passos →](02-primeiros-passos.md)

## Para que serve

Imagine que você tem um projeto de software e quer fazer uma mudança: corrigir um texto, criar uma tela, arrumar um erro. Em vez de escrever o código, você **descreve o que quer** e um agente de IA faz a mudança. O Harnessy é o painel de controle que organiza isso com segurança:

- guarda suas **tarefas** num quadro;
- manda o agente trabalhar numa **cópia separada** do projeto, para o original ficar intacto;
- **confere** o resultado (roda os testes, uma revisão automática e mais);
- deixa você **ler** o que mudou e **testar** no seu Mac;
- **entrega** para o GitHub e conclui, quando você mandar;
- **avisa** você só quando precisa de você.

Tudo roda **no seu Mac**. Seus dados de trabalho (tarefas, histórico) ficam num arquivo local.

## As ideias principais

| Ideia | Em palavras simples |
|---|---|
| **Workspace** | A pasta do seu projeto. Você pode ter vários |
| **Tarefa** | Um pedido: título, descrição e **critérios de aceite** ("está pronto quando…") |
| **Quadro** | As tarefas em quatro colunas: *A fazer*, *Em andamento*, *Revisão*, *Concluído* |
| **Run** | Uma tentativa do agente de fazer a tarefa. Cada run tem registro, tempo e custo |
| **Branch** | Uma versão paralela do projeto onde a mudança é feita sem mexer na original |
| **PR** (pull request) | O pedido para incluir a mudança no projeto principal, no GitHub, onde pode ser conferido |
| **Draft** | Um PR "rascunho": avisa que ainda não está para valer |
| **Merge** | Incluir a mudança no projeto principal. É o passo final |

Mais palavras no [glossário](glossario.md).

## O que o Harnessy decide e o que decide você

O Harnessy foi feito para que **você fique no controle com o mínimo de esforço**:

- Passos que **saem da sua máquina** (enviar ao GitHub, fazer merge) só acontecem quando você manda.
- Antes de rodar um comando que vem do projeto, ele **mostra o comando e pede sua confirmação**.
- Você sempre vê **por que** algo está parado e **qual é o próximo passo**.

## O que você precisa saber para começar

- Um Mac (Apple Silicon, no momento).
- O **Claude Code** instalado e com login feito (é o agente).
- O **git** e o **GitHub CLI (`gh`)** instalados e com login, se for publicar no GitHub.

Não precisa saber programar para usar o quadro, mas **entender o que a mudança faz** ajuda muito na hora de conferir. Se algo parecer estranho, não aprove: peça um ajuste.

Próximo: [Primeiros passos →](02-primeiros-passos.md)
