# 6. Testar localmente

[← Guia](../user-guide.md) · [← Anterior](05-revisao-e-alteracoes.md) · [Próximo: Publicar e fazer merge →](07-publicar-e-merge.md)

## Para que serve

Ler as alterações mostra **o que** mudou; só usar mostra **se funciona**. O botão **Rodar localmente** liga o projeto **no seu Mac, com a mudança do agente**, para você abrir, clicar e ver. No futuro, a mesma ideia servirá para um ambiente de QA na nuvem.

## Como usar

1. Selecione a tarefa. Se o próximo passo for testar, o botão principal é **Rodar localmente**. Ele também está em **Mais ações**.
2. **Na primeira vez**, o Harnessy pergunta **"Como subir este projeto?"**:
   - **Comando**: o que você digitaria no Terminal para ligar o projeto (por exemplo, `bun run dev` ou `npm run dev`). O Harnessy já sugere um, lendo o `package.json`.
   - **Porta** (opcional): o "número da porta" em que o projeto abre, como `5173`. Sem ela, o Harnessy descobre o endereço lendo o que o comando escreve.
3. Clique em **Salvar e rodar**. O comando é gravado em `.harnessy/workspace.toml`, dentro do projeto, e confirmado por você.
4. Acompanhe no painel **Ambiente de teste**:

| Estado | Significa |
|---|---|
| **Subindo…** | Está ligando (inclui recriar a cópia e instalar dependências, se for preciso) |
| **No ar** | Pronto: aparece o endereço, como `http://localhost:5173` |
| **Parado** | Foi desligado |
| **Caiu (código N)** | O comando terminou com erro; veja o log |

5. Com o ambiente **No ar**, clique em **Abrir no navegador** e teste de verdade.
6. Quando terminar, clique em **Parar**. Para ligar de novo, **Rodar de novo**.

O card mostra o selo **No ar** enquanto estiver rodando, e o painel guarda a saída do comando (o **log**) numa área que você pode rolar com o teclado.

## Quando o comando vem do projeto

Se o projeto **já traz** um `.harnessy/workspace.toml` (por exemplo, de um colega), o Harnessy **mostra o comando e pede sua confirmação** antes de rodar. Isso protege você: um projeto baixado de qualquer lugar não executa nada sem você ver.

A confirmação vale para **aquele texto exato**. Se o arquivo for editado, o Harnessy pede de novo.

## Regras úteis

- Só **um** ambiente fica no ar por vez, para não brigar por porta. Ligar outro desliga o anterior.
- O ambiente é **desligado sozinho** ao fechar o Harnessy ou excluir a tarefa.
- Ele roda **numa cópia da run**, não no seu projeto original; por isso reflete exatamente a mudança do agente. A cópia que o agente usou é apagada quando a run termina, então o Harnessy **a recria a partir da branch** na hora de rodar (o painel mostra "recriando…" e "instalando as dependências…", e isso pode levar alguns minutos na primeira vez) e a **remove de novo** ao parar.
- O comando roda sem as suas senhas e tokens: o Harnessy usa só um conjunto fixo e seguro de variáveis do ambiente.
- O botão **Abrir no navegador** só abre endereços locais (`http://localhost:…`) que o próprio ambiente anunciou.

## Se algo der errado

| Sintoma | O que fazer |
|---|---|
| Fica em **Subindo…** | O comando não imprimiu um endereço e você não informou a porta. Veja o log; se o projeto abre numa porta fixa, informe-a e rode de novo |
| **Caiu (código 1)** | Leia o final do log. Muitas vezes falta instalar algo ou a porta já está em uso |
| "Porta já em uso" | Outro programa usa a porta. Troque a porta no comando ou feche o outro programa |
| Falha ao "recriar a cópia" | A branch da run não existe mais no git (foi apagada) ou já está aberta em outro lugar. Leia o motivo no log do painel |
| Pede confirmação de novo | O arquivo `.harnessy/workspace.toml` mudou desde a última confirmação |

Próximo: [Publicar e fazer merge →](07-publicar-e-merge.md)
