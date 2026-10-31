# 5. Revisão e alterações

[← Guia](../user-guide.md) · [← Anterior](04-agentes-e-runs.md) · [Próximo: Testar localmente →](06-testar-localmente.md)

## Para que serve

Antes de aceitar uma mudança, você precisa **entender o que mudou** e **confiar que está certo**. O Harnessy ajuda com quatro coisas, da mais mecânica à mais humana:

| O quê | Quem faz | Para que serve |
|---|---|---|
| **Verificações** | O Harnessy (testes e checagens do projeto) | Garantir que nada quebrou |
| **Contraprova** | O Harnessy | Garantir que os testes novos de fato pegam a mudança |
| **Revisão automática** | Um segundo agente, como um *staff engineer* | Opinar sobre a qualidade do código |
| **Sua leitura** | **Você** | A decisão final |

## Ler o que mudou

Selecione a tarefa e abra **Alterações da run** (fica recolhido, e só carrega quando você abre).

1. Veja o resumo, por exemplo `3 arquivos · +40 −12` (40 linhas adicionadas, 12 removidas).
2. Na lista, cada arquivo mostra o estado (**novo**, **alterado**, **removido**, **renomeado**) e a contagem de linhas. Clique para ver aquele arquivo.
3. No texto do arquivo:
   - linhas com **+** foram **adicionadas**; com **−**, **removidas**; as sem sinal são só contexto;
   - os dois números à esquerda são a linha antes e depois;
   - as cores ajudam, mas o sinal `+`/`−` sempre está lá.
4. Se a revisão automática comentou alguma linha, o comentário aparece **logo abaixo dela**, marcado **Bloqueia** ou **Sugere**.

O app abre direto no primeiro arquivo que a revisão comentou. Arquivos binários (como imagens) e arquivos muito grandes avisam que não mostram tudo.

> **Pergunta de ouro:** "Eu entendi o que isso faz e é o que eu pedi?" Se não, peça um ajuste em vez de aprovar.

## A revisão automática (o "staff engineer")

É um segundo agente que lê o resultado **sem ver a conversa de quem implementou**, só a tarefa, os critérios e as alterações. Ele foi instruído a pensar como um engenheiro sênior:

- a solução é **simples** e proporcional ao pedido, ou tem camadas que ninguém pediu?
- segue as **regras e a arquitetura do projeto**?
- dá para **testar** e **manter**?
- trata bem quem usa o produto?

Ele devolve um veredito (**aprovar** ou **pedir mudanças**) e até 10 achados:

| Achado | Significa |
|---|---|
| **Bloqueia** | Violação de uma regra documentada, complexidade claramente desproporcional ou defeito evidente. Vale corrigir antes de seguir |
| **Sugere** | Melhoria opcional |

O Harnessy **descarta** automaticamente qualquer achado que não aponte para uma linha realmente alterada, para evitar comentários inventados. Se um achado foi descartado, a conversa avisa quantos.

**Rodar de novo ou sob demanda:** o botão **Revisar agora** (ou **Revisar de novo**) pede a revisão mesmo sem sinal de risco, com o revisor que você escolheu.

> A revisão é uma **opinião**, não uma garantia. Ela ajuda, mas não substitui a sua leitura.

## A contraprova

Um teste que passa **mesmo sem a mudança** não prova nada. A contraprova roda os testes novos contra o projeto **sem** a mudança:

| Resultado | Significa |
|---|---|
| **Provada** | Os testes novos falham sem a mudança: eles de fato verificam algo |
| **Não provada** | Os testes passam mesmo sem a mudança: podem não verificar nada |
| **Sem testes** | A run não criou testes |

É um **aviso**, não um bloqueio, e não gasta nada do agente.

## Pedir mudanças

Se algo não ficou bom, abra o **Chat** da tarefa e descreva o ajuste. O agente continua da **mesma branch**. A aprovação anterior da revisão deixa de valer quando o código muda, e uma nova revisão é feita.

## Se algo der errado

| Sintoma | O que fazer |
|---|---|
| "Alterações da run" mostra erro | A branch pode ter sido apagada no git. Veja se ela ainda existe |
| Não aparecem comentários da revisão | A revisão pode ter sido **dispensada** por ser uma mudança pequena e sem risco (a conversa diz isso), ou não ter achado nada. Use **Revisar agora** para forçá-la |
| A revisão "não devolveu o formato esperado" | Tente **Revisar de novo** ou escolha outro revisor |

Próximo: [Testar localmente →](06-testar-localmente.md)
