# 4. O agente e as runs

[← Guia](../user-guide.md) · [← Anterior](03-tarefas-e-quadro.md) · [Próximo: Revisão e alterações →](05-revisao-e-alteracoes.md)

## Para que serve

Uma **run** é uma tentativa do agente de fazer uma tarefa. Esta página explica como iniciá-la, o que acontece durante, como pedir ajustes e como acompanhar tempo e custo.

## O que acontece quando você manda implementar

1. O Harnessy cria uma **cópia separada** do projeto (um *worktree*, numa *branch* nova chamada `harnessy/run-…`). O seu projeto original **não é tocado**.
2. Instala o que o projeto precisa nessa cópia.
3. O agente lê a tarefa e as **regras do projeto** (veja o quadro abaixo) e escreve o código.
4. Quando o agente termina, o Harnessy **roda as verificações do projeto** (testes, checagem de tipos, etc.). Se algo falhar, o agente recebe o erro e tenta corrigir, **até 3 vezes**.
5. O Harnessy faz uma **contraprova** (veja abaixo) e a **revisão automática**.
6. O card vai para **Revisão** e você é avisado.

Uma run tem **limite de 15 minutos** e de turnos do agente, para não ficar rodando sem fim. Você pode **cancelar** a qualquer momento no registro da run.

### As regras do projeto

Muitos projetos têm um manual de regras escrito para agentes (arquivos como `AGENTS.md`, `CLAUDE.md`, `.claude/rules` e as decisões em `docs/adr`). O Harnessy **lê esses arquivos por conta própria**, a partir da versão do projeto **antes** da mudança, e os entrega ao agente como **informação**: o agente segue as convenções do projeto, mas esses arquivos **não conseguem mudar as regras de segurança do Harnessy**. Para ver exatamente o que o agente vai receber, abra **Prompt que o agente receberá** no detalhe da tarefa, antes de iniciar.

## Escolher o agente e o modelo

No seletor de agente você escolhe:

| Campo | O que é |
|---|---|
| **Agente** | Quem implementa. Hoje: **Claude Code**. O Codex aparece, mas ainda não implementa |
| **Modelo** | O "tamanho" do cérebro: modelos maiores resolvem coisas mais difíceis, custam mais e demoram mais. O padrão é **Sonnet** |
| **Revisor** | Quem revisa o resultado: Claude Code (um modelo acima do que implementou, por padrão) ou Codex |
| **Quando revisar** | **Sempre** (padrão), **Só com risco** ou **Nunca** |

A escolha fica salva no seu Mac e vale para todas as tarefas.

> Quando uma run **falha** ou o resultado não convence, o botão **Tentar com** o próximo modelo repete o trabalho com um modelo maior, sem perder o que já foi feito.

## Acompanhar o trabalho: o Chat

Cada tarefa tem uma **conversa** no **Chat** (`⌘2`), que reúne **todas as runs dela**, em ordem. Ali você vê o que o agente fez e disse, o resultado das verificações e da revisão, e o campo para pedir mais.

- **Peça um ajuste** nesta run: escreva o que quer mudar (até 4.000 caracteres). O agente **continua da mesma branch**, sem refazer o que já está certo.
- **Logs técnicos** ficam recolhidos; abra só se quiser os detalhes.
- Uma linha por tarefa na lista da esquerda, com quantas runs ela tem.

## Tempo, tokens e custo

No detalhe da run há um resumo honesto do consumo: **tempo do agente**, tempo das **verificações**, **tokens** (as "palavras" que o modelo leu e escreveu) e **custo estimado**. Quando o agente não informa um valor, o Harnessy mostra "não informado" em vez de inventar um zero.

## Outras ações

No detalhe da tarefa, **Mais ações** reúne outras formas de usar o agente:

| Ação | Para que serve |
|---|---|
| **Explicar** | O agente só **lê** o projeto e responde sobre a tarefa. Não muda nada |
| **Dividir em passos** | Propõe tarefas menores e encadeadas |
| **Nova Ação** | Crie a sua: um pedido reutilizável (nome, o que pedir, só ler ou também escrever, modelo) |

Projetos podem trazer ações próprias em `.harnessy/actions/`. Antes de a primeira ser usada, o Harnessy mostra **Confirmar Ação do repositório**; se o arquivo mudar depois, pede de novo.

## Se algo der errado

| Sintoma | O que fazer |
|---|---|
| O botão de implementar está desabilitado | O texto ao lado diz o que falta: criar um workspace, salvar a tarefa, ou instalar Claude Code/git. Veja **Conexões** |
| A run ficou **Bloqueada** | Leia o motivo no card. Peça um ajuste, tente com um modelo maior, ou **Dispensar aviso** |
| "Há uma run ativa neste workspace" | Só uma run roda por vez em cada workspace. Espere ou cancele |
| Falha de teste com erro de Node | Veja a [solução de problemas](../troubleshooting.md) |

Próximo: [Revisão e alterações →](05-revisao-e-alteracoes.md)
