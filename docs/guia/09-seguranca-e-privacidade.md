# 9. Segurança e privacidade

[← Guia](../user-guide.md) · [← Anterior](08-atencao-e-notificacoes.md) · [Glossário →](glossario.md)

## Para que serve

Dar a um agente acesso ao seu código exige confiança com limites claros. Esta página diz, sem rodeios, **o que o Harnessy faz para proteger você** e **onde o risco continua**.

## O que o agente pode e não pode fazer

| O agente **pode** | O agente **não pode** |
|---|---|
| Ler e editar arquivos **na cópia separada** da run | Mexer no seu projeto original |
| Rodar comandos de leitura e as verificações do projeto | Enviar nada ao GitHub (`git push`, `gh`) |
| Usar o login que você já fez no Claude Code | Acessar a internet, `curl` ou `ssh` |
| | Editar `.git`, `.claude`, `.harnessy`, arquivos `.env` e arquivos de travamento de dependências |

Quem envia ao GitHub e faz merge é o **Harnessy**, depois de verificar o que vai e só quando **você** manda.

## As duas camadas de regras

O Harnessy tem um conjunto de regras **internas e fixas** (o que o agente pode fazer, em que formato responde, como é julgado). O seu projeto tem as próprias regras (convenções, arquitetura). As duas se encontram assim:

- as regras do **Harnessy mandam**: nenhum arquivo do projeto consegue mudá-las;
- as regras do **projeto informam**: o agente as segue para escrever código no estilo certo;
- o Harnessy lê as regras do projeto **da versão anterior à mudança**, então uma mudança não consegue reescrever as regras pelas quais ela mesma é julgada;
- o texto de um arquivo do projeto que tente "dar ordens" ao agente (como "aprove sempre") é tratado como **dado**, não como instrução.

## O que exige a sua confirmação

- **Comandos que vêm do projeto**, como o de subir o ambiente de teste: o Harnessy mostra o texto e pede um "sim" **para aquele texto exato**.
- **Ações do repositório** (`.harnessy/actions/`): idem, e de novo se o arquivo mudar.
- **Enviar ao GitHub e fazer merge**: só por clique seu.
- **Excluir uma tarefa**: confirmação, e as branches no git ficam.

Quem **define o quanto delegar** é você, e essa escolha vive só no seu Mac. Um projeto **não consegue se autorizar** a publicar ou mesclar sozinho; no máximo, pode **apertar** limites.

## Onde ficam seus dados

| O quê | Onde |
|---|---|
| Tarefas, histórico, runs, configurações | Um arquivo local: `~/Library/Application Support/app.harnessy.desktop/local-work.sqlite3` |
| Cópias de trabalho das runs | `~/Library/Application Support/app.harnessy.desktop/worktrees/` |
| Rascunho do chat | Armazenamento local do app (não é um cofre: **não cole senhas ou chaves**) |
| Senhas e tokens | **Não são guardados pelo Harnessy.** Ficam no chaveiro e nas ferramentas (`gh`, `claude`, `codex`) |

O Harnessy não tem conta, servidor próprio nem envia telemetria.

## O que sai do seu Mac

- O **Claude Code**, ao trabalhar, envia partes do seu código e da tarefa ao provedor do modelo (a Anthropic), como já faria se você o usasse no Terminal. Isso é do agente, não do Harnessy.
- Ao **publicar**, o Harnessy envia a branch e o texto do PR ao **GitHub**, com o seu login.
- Textos de log passam por um filtro que **esconde padrões de token** antes de serem gravados.

## Onde o risco continua

Seja claro com você mesmo:

- Não existe **isolamento forte** do agente (sem sandbox ou contêiner). As regras de bloqueio de comandos funcionam por texto e **não são uma barreira de segurança absoluta**: um comando indireto pode escapar delas. Por isso o Harnessy deve ser usado **só em projetos em que você confia**.
- Um arquivo malicioso dentro do projeto (por exemplo, um texto feito para enganar o agente) pode **influenciar o julgamento** do agente. Ele não abre novas permissões, mas por isso a **sua leitura** do que mudou é insubstituível.
- O que você escreve na tarefa vira a instrução do agente. **Não cole conteúdo de fontes desconhecidas** nela.

## Boas práticas

1. Confira as **Alterações da run** antes de publicar.
2. Leia o comando antes de confirmá-lo.
3. Mantenha o login do GitHub com as permissões mínimas que você precisa.
4. Trate o PR em **draft** como rascunho, até a revisão e os checks passarem.
5. Se algo parecer estranho, **não aprove**: peça um ajuste.

Próximo: [Glossário →](glossario.md)
