# Roteiro de teste: o Harnessy rodando no próprio Harnessy

[← README](../README.md) · [Desenvolvimento](development.md) · [Changelog](../CHANGELOG.md)

Objetivo: percorrer o ciclo inteiro uma vez, **como usuário**, e anotar onde o fluxo trava, confunde ou funciona sem esforço. O que vale aqui não é só "funcionou": é **onde você hesitou**.

Tempo estimado: 60 a 90 minutos (a espera do agente e do CI é a maior parte).

> **Pense em voz alta.** Cada vez que você não souber o que fazer agora, anote o passo. Isso mede a promessa do produto ("sempre uma próxima ação óbvia") melhor que qualquer teste automatizado.

## O que está e o que não está neste teste

| Está pronto para testar | Ainda não existe (não é bug) |
|---|---|
| Criar tarefa com critérios, prévia do prompt, implementar | Interface e motor de **automação** (modos por etapa): só a base no núcleo |
| Revisão do staff engineer e achados no diff | **QA** como papel separado |
| Alterações da run no app | **Discovery** (preparar o harness de um projeto novo) |
| Ação principal ("Próximo passo") e tecla `A` | Abrir o PR no navegador por botão |
| Rodar localmente, painel e log do ambiente | Revisão comentando **dentro do PR** |
| Publicar PR em draft, estágios, merge com confirmação | PR cuja base não é a branch padrão (veja o aviso do passo 0) |
| Filtro "Precisa de você", bandeja, notificação, resumo de ausência | |

## 0. Preparação: uma instância separada para o teste

Testar o Harnessy com o próprio Harnessy tem uma armadilha: o app que você usa para trabalhar e o app em teste seriam **o mesmo** (mesmo banco, mesmo ícone, mesma porta). Por isso o teste roda numa **segunda instância**, com identidade própria:

| | Seu Harnessy principal | Instância de teste |
|---|---|---|
| Nome | Harnessy | **Harnessy Dogfood** |
| Dados | `~/Library/Application Support/app.harnessy.desktop` | `~/Library/Application Support/app.harnessy.desktop.dogfood` |
| Ícone na barra | **H** | **H·dogfood** |
| Branches das runs | `harnessy/run-N-…` | `harnessy-dogfood/run-N-…` |
| Faixa na tela | nenhuma | **Instância de teste · dogfood** |

**Você não precisa fechar o app principal nem fazer backup**: nada do que acontece na instância de teste toca o seu banco.

1. Gere e abra a instância de teste, **na pasta do repositório**:
   ```sh
   bun install --frozen-lockfile
   bun run app:dogfood
   ```
   A primeira vez compila o núcleo do zero (alguns minutos, num diretório próprio que não atrapalha o build normal). Depois é incremental.
2. Confirme a faixa **Instância de teste · dogfood** no topo e o **H·dogfood** na barra de menus. Se não vê isso, você está no app errado.
3. Em **Conexões**, confirme `gh`, `claude` e `git` verdes. Sem `gh` logado, os passos 7 a 9 não funcionam.
4. Crie um workspace apontando para esta pasta do Harnessy. Os dois apps podem trabalhar no mesmo repositório sem se atrapalhar, porque cada um tem seu banco e seu prefixo de branch.
5. **Decida o destino do PR de teste.** Esta branch tem commits que não estão no GitHub, e o PR da run vai mostrar tudo isso junto. O **merge (passo 9) levaria esses commits para a `main` do GitHub**.
   - **Opção A (recomendada):** faça você mesmo o merge desta branch na `main` e dê o push, e só então teste, sobre a `main`.
   - **Opção B:** teste na branch e **pare no passo 8**. Feche o PR depois e apague a branch remota (passo 12).

- [ ] Instância de teste aberta, com a faixa e o **H·dogfood**
- [ ] Conexões verdes
- [ ] Workspace do Harnessy criado na instância de teste

## 1. Primeira impressão do quadro

1. Olhe o quadro vazio e a barra lateral. O que você acha que deve fazer primeiro? Anote.
2. Observe o ícone da bandeja: deve ser **"H"** (sem número).
3. Com tarefas existentes, veja a barra **Todas / Precisa de você / Com os agentes**.

Esperado: tela sem erro; o filtro só aparece quando há tarefas.

## 2. Criar a tarefa de teste

Use uma tarefa **só de documentação**: segura e fácil de revisar.

- **Título:** `Acrescentar "Dispensar aviso" ao glossário do guia`
- Clique em **+ Descrição e critérios** e preencha:
  - **Descrição:** `O docs/guia/glossario.md explica as palavras do Harnessy, mas não tem o botão "Dispensar aviso", que aparece numa tarefa Bloqueada. Acrescente uma linha na tabela, no mesmo estilo das outras e em ordem alfabética.`
  - **Critérios:** `- Existe uma linha "Dispensar aviso" entre "Critérios de aceite" e "Draft".` / `- A explicação diz que ela tira o aviso de bloqueio sem mudar a coluna da tarefa.` / `- Nenhum outro arquivo é alterado.`

Esperado: a tarefa aparece em **A fazer**. Tecle `n` em outra tela do quadro: o foco deve ir ao campo de título.

- [ ] Criada com critérios

## 3. Prévia do prompt (a prova das duas camadas de harness)

Abra a tarefa e veja o prompt do agente antes de iniciar.

Confira:
- [ ] Existe um bloco **"Padrões do projeto"** com `<harness-do-projeto arquivo="AGENTS.md" sha256="…">` e também os arquivos de `.claude/rules` e `docs/adr`.
- [ ] O texto diz que são **dados** e que as regras do Harnessy valem em caso de conflito.
- [ ] **Não** existe mais a frase "Siga o AGENTS.md" solta nas regras.
- [ ] O bloco vem **depois** das regras do Harnessy e **antes** de "# Tarefa".

## 4. Implementar e não ficar vigiando

1. Com a tarefa selecionada, o **Próximo passo** deve ser **Implementar**. Tecle `A` (ou clique).
2. O card vai para **Em andamento** com "agente trabalhando", e o painel diz que o agente trabalha e que você será avisado.
3. **Troque para outro aplicativo** (navegador, por exemplo) e espere. Não fique olhando.
4. Quando a run terminar:
   - [ ] O ícone da bandeja mostra **"H 1"**.
   - [ ] O macOS mostra uma notificação "Precisa de você". Se ela não aparecer, confira Ajustes do Sistema → Notificações (em modo dev, o macOS pode atribuí-la ao terminal) e, se preciso, repita com `bun run desktop:build`.
5. Volte para a janela:
   - [ ] O banner **"Enquanto você esteve fora"** lista a tarefa com o motivo e dois botões.
   - [ ] **Ver o que precisa de mim** ativa o filtro "Precisa de você".

Esperado: card em **Revisão**. Anote quanto tempo a run levou e se algo pareceu parado sem explicação.

## 5. Ler o que mudou (o coração do "humano no loop")

1. Abra **Alterações da run**.
2. Confira:
   - [ ] Resumo (`N arquivos · +X −Y`) coerente com o que a tarefa pediu.
   - [ ] **Só** `docs/guia/glossario.md` foi alterado. Se mais arquivos mudaram, o critério 3 foi violado: anote.
   - [ ] **Nenhum bump de versão** nem entrada de CHANGELOG (a regra de versionamento diz que "só docs" não bumpa). Isso mostra se o harness do projeto foi respeitado.
   - [ ] Números de linha dos dois lados e `+`/`−` legíveis; o patch rola por teclado.
3. Se a revisão do staff rodou, os achados aparecem **na linha**, com "Bloqueia" ou "Sugere". Os achados são úteis ou ruído? São sobre simplicidade, padrões e manutenção (staff), e não sobre bugs? Anote exemplos bons e ruins.

## 6. Rodar localmente

O repositório traz o comando certo no `.harnessy/workspace.toml`: `bun run app:preview {port}`. Ele abre **uma terceira instância**, a **Harnessy Preview**, com a cópia da run, identidade própria e a porta que o Harnessy escolher. É a forma de ver a mudança do Harnessy funcionando **dentro do app de verdade** (o front sozinho, no navegador, não tem dados).

1. O Próximo passo deve ser **Rodar localmente**. Execute.
2. Como o comando veio do repositório, o Harnessy **mostra o texto e pede a sua confirmação**. Confira que é `bun run app:preview {port}` e confirme.
3. Confira:
   - [ ] O painel passa por **Subindo…** e o log mostra a cópia sendo recriada e as dependências preparadas. **A primeira vez compila o núcleo Rust, o que leva alguns minutos**; nas seguintes é bem mais rápido.
   - [ ] Uma janela **Harnessy Preview** abre sozinha, com a faixa **Instância de teste · preview** e o **H·preview** na barra.
   - [ ] O painel chega a **No ar** e o card mostra o selo **No ar**.
   - [ ] A porta **não é a 1420** (o log mostra a escolhida) e **não há conflito** com a instância de teste.
4. Use a janela Preview para ver a mudança. Clique em **Parar** no painel: a janela Preview fecha e a porta é liberada (`lsof -i :PORTA` não mostra nada).
5. Edite o `.harnessy/workspace.toml` (qualquer mudança no comando) e rode de novo: o app deve **pedir nova confirmação**.

Depois do ambiente testado, o Próximo passo deve ser **Publicar PR**.

## 7. Publicar o PR (efeito real no GitHub)

> Isto faz `git push` da branch `harnessy/run-…` para o GitHub e abre um PR **em draft**.

1. Execute **Publicar PR**.
2. No GitHub, confira:
   - [ ] O PR está em **draft** e o título é o da tarefa.
   - [ ] O corpo tem as seções Tarefa, Critérios de aceite, Resumo da run e Verificações, e termina com "Publicado pelo Harnessy · run #N".
   - [ ] Nenhum arquivo parecido com segredo foi enviado.
3. No app, o selo muda para **PR em draft** (depois **Checks rodando**, porque há CI).
4. Clique em **Atualizar PR** de novo: **não** deve criar um segundo PR.
5. Espere o CI. O selo deve ir para **Checks falhando** ou ficar apto a avançar. Se falhar, o Próximo passo vira **Pedir ajuste**: teste pedir um ajuste e veja se a run continua da mesma branch.

## 8. Revisar e chegar a "Pronto para merge"

1. Quando o selo pedir, execute **Revisar** (o Próximo passo). Leia o veredito.
2. Com checks sem falha, sem conflito e revisão aprovada **neste commit**, o selo deve ser **Pronto para merge** e o Próximo passo, **Fazer merge**.
3. **Prove a invalidação:** faça um commit novo na branch do PR (peça um ajuste) e veja o selo **voltar** para antes de "Pronto" até uma nova revisão.

> **Opção B: pare aqui.** Pule o passo 9.

## 9. Merge (efeito real e irreversível na `main`)

1. Execute **Fazer merge**. Leia o texto da confirmação: ele explica squash, o destino e que a tarefa vai para Concluído.
2. Depois de confirmar:
   - [ ] O PR aparece como **mergeado** no GitHub (squash).
   - [ ] O card vai para **Concluído** e o selo vira **Mergeado**.
   - [ ] A branch local continua existindo (o worktree da run ainda a usa).

## 10. Cenários de borda

- [ ] **Cancelar** uma run em andamento: o card volta de forma coerente e o bloqueio não aparece indevidamente.
- [ ] **Fechar a janela** (vai para a bandeja) durante uma run e esperar: a notificação ainda chega?
- [ ] **Excluir a tarefa** com o ambiente de teste no ar: o ambiente é parado e a porta fica livre.
- [ ] **Fechar o app** com o ambiente no ar (Encerrar na bandeja): nenhum `vite` órfão (`pgrep -fl vite`).
- [ ] **Sem rede ou sem `gh` logado** ao atualizar o PR: a mensagem de erro diz o que fazer?

## 11. Teclado, acessibilidade e tema

- [ ] Percorra o quadro só com o teclado: `Tab`, `n`, `a`, `[`, `]`, `1` a `4`. O foco é sempre visível?
- [ ] Ative **Reduzir movimento** (Ajustes do Sistema → Acessibilidade → Tela): o ícone de "checks rodando" deve parar de girar.
- [ ] Alterne o tema claro e escuro: contraste do selo, do diff e do banner continuam legíveis?
- [ ] Se usar o VoiceOver: ao mudar o estágio de uma tarefa, a frase anunciada é completa ("Tarefa #N: …")?

## 12. Desfazer o teste

Nada do teste tocou o seu Harnessy principal, então não há o que restaurar.

```sh
# apagar os dados da instância de teste e da preview (opcional)
rm -rf ~/Library/Application\ Support/app.harnessy.desktop.dogfood
rm -rf ~/Library/Application\ Support/app.harnessy.desktop.preview

# limpar cópias de trabalho e a branch remota do teste (troque NOME)
git worktree prune
git push origin --delete harnessy-dogfood/run-NOME   # só se quiser descartar o PR
```

## 13. Como reportar

Para cada problema, uma linha:

| Passo | Esperado | Observado | Gravidade |
|---|---|---|---|
| 5.2 | Só 1 arquivo alterado | 3 arquivos | Alta |

Gravidade: **Alta** (fluxo travou ou efeito errado), **Média** (confuso, mas dá para continuar), **Baixa** (polimento). Inclua prints e, se houver erro, o texto exato. O terminal do `bun run tauri dev` mostra o log do núcleo; a conversa da run tem **Logs técnicos**.

## 14. As cinco perguntas finais

1. Em quantos momentos você **não soube o que fazer em seguida**?
2. Você confiou no que aprovou, **vendo o diff** sem sair do app?
3. Você **esqueceu** do Harnessy enquanto o agente trabalhava, e ele te chamou na hora certa?
4. Qual etapa você **delegaria** ao agente hoje, e qual manteria na mão? (Isso define os padrões do motor de automação.)
5. Você usaria isto amanhã no lugar do seu fluxo atual? O que faltaria?
