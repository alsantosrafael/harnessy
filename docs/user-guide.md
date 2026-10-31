# Guia do Harnessy

[← README](../README.md) · [Instalação](getting-started.md) · [Solução de problemas](troubleshooting.md)

O Harnessy é um aplicativo para Mac que ajuda você a tocar um projeto de software junto com agentes de IA (hoje, o Claude Code). Você descreve o que quer, o agente faz a mudança, e o Harnessy cuida do resto: confere o resultado, deixa você ver e testar, e entrega para o GitHub. **Você decide em cada passo importante; o agente nunca mexe no seu projeto principal nem publica nada sozinho.**

Este guia foi escrito para quem **não é da área técnica**. Cada assunto tem uma página própria, e você pode ler só a que precisa.

## O caminho de uma tarefa

```text
 1. Você         2. Agente        3. Você          4. Você         5. Entrega       6. Você
 descreve    →   faz a mudança →  confere o    →   testa no    →   vira um PR   →   aprova o
 a tarefa        (e é checado)    que mudou        seu Mac         no GitHub        merge
```

Em todos os passos, o Harnessy mostra **qual é o próximo passo** e por quê. Se você ficar perdido, procure o botão **Próximo passo** na tarefa.

## Quem faz o quê

| Quem | O que faz | O que **nunca** faz |
|---|---|---|
| **Você** | Descreve a tarefa, lê as alterações, testa, decide publicar e fazer merge | — |
| **O agente** (Claude Code) | Escreve o código numa cópia separada do projeto e roda os testes | Enviar nada ao GitHub, mexer no seu projeto principal, acessar a internet |
| **O Harnessy** | Prepara a cópia separada, confere o resultado, revisa, avisa você, publica e mescla **quando você manda** | Agir sem você nos passos que saem da sua máquina |

## Os guias

| Quero… | Leia |
|---|---|
| Entender o que é o Harnessy e as palavras que ele usa | [1. O que é o Harnessy](guia/01-o-que-e-o-harnessy.md) |
| Instalar, conectar e fazer a primeira tarefa | [2. Primeiros passos](guia/02-primeiros-passos.md) |
| Organizar o trabalho no quadro | [3. Tarefas e o quadro](guia/03-tarefas-e-quadro.md) |
| Mandar o agente trabalhar e acompanhar | [4. O agente e as runs](guia/04-agentes-e-runs.md) |
| Conferir o que o agente fez | [5. Revisão e alterações](guia/05-revisao-e-alteracoes.md) |
| Ver a mudança funcionando no meu Mac | [6. Testar localmente](guia/06-testar-localmente.md) |
| Enviar para o GitHub e concluir | [7. Publicar e fazer merge](guia/07-publicar-e-merge.md) |
| Ser avisado quando algo precisa de mim | [8. Atenção e notificações](guia/08-atencao-e-notificacoes.md) |
| Saber o que é seguro e o que fica no meu Mac | [9. Segurança e privacidade](guia/09-seguranca-e-privacidade.md) |
| Entender uma palavra | [Glossário](guia/glossario.md) |

## Comece em 5 minutos

1. [Abra o aplicativo](guia/02-primeiros-passos.md#1-abrir-o-aplicativo) e confira em **Conexões** se o Claude Code, o `git` e o GitHub (`gh`) estão **Conectados**.
2. [Crie um workspace](guia/02-primeiros-passos.md#3-criar-um-workspace): é a pasta do seu projeto.
3. Em **Tarefas**, escreva um título na coluna **A fazer** e tecle `Enter`.
4. Selecione a tarefa e clique no botão **Próximo passo** (ou tecle `A`).
5. Quando o Harnessy avisar, abra **Alterações da run** e leia o que mudou.

## O que ainda não existe

Para você não procurar o que não está lá:

| Ideia | Situação |
|---|---|
| Deixar etapas rodarem sozinhas (modos "eu conduzo", "assistido", "delegado") | A base existe no núcleo; **ainda não há tela**. Hoje você conduz cada passo |
| Um agente de QA que testa casos de uso e bordas | Planejado |
| Preparar automaticamente o "manual de regras" de um projeto novo (discovery) | Planejado |
| Abrir o PR no navegador com um botão | Planejado (o endereço aparece no registro da run) |
| Merge automático | Não existe e, por decisão de segurança, não será ligado sem sua escolha explícita |

Veja o que mudou em cada versão no [changelog](../CHANGELOG.md).
