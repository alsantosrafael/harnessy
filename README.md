# Harnessy

**Um espaço local para organizar trabalho, conversar com agentes e acompanhar entregas.**

Aplicativo desktop feito com **Tauri 2, Rust, React e TypeScript**, usando **Bun, Vite e Vitest** no frontend. Primeira plataforma: **macOS**, validado inicialmente em Apple Silicon.

> **Em desenvolvimento.** Crie tarefas, mande o Claude Code implementar numa cópia separada do projeto, leia as alterações, teste no seu Mac, publique um PR em draft e faça o merge, tudo com o Harnessy dizendo qual é o próximo passo e avisando quando precisa de você. O número da versão está no [changelog](CHANGELOG.md).

## Comece aqui

### Já tem o build neste projeto?

Na pasta raiz do projeto, execute:

```bash
bun run app
```

Ele abre `src-tauri/target/release/bundle/macos/Harnessy.app` (e gera o build antes, se ainda não existir). Para recompilar com o código atual, use `bun run app:build`; para copiar para `~/Applications` e abrir de lá, `bun run app:install`. Também dá para abrir `Harnessy.app` pelo Finder nesse caminho. O aplicativo compilado não precisa de um terminal ou servidor Vite em execução.

### Quer executar pelo código-fonte?

Você precisa de **Bun 1.4.3+, Rust estável e Xcode Command Line Tools**. Para o runner Vitest, mantenha também Node.js 22+ disponível. Se ainda não preparou essas ferramentas, siga o [guia de instalação](docs/getting-started.md).

No terminal, entre na pasta do projeto e execute:

```bash
bun install --frozen-lockfile
bun run start
```

O comando abre a janela desktop e inicia automaticamente o frontend de desenvolvimento. A primeira compilação Rust pode demorar alguns minutos; acompanhe o terminal.

## Primeiro passeio

1. Abra **Conexões** (`⌘3`) e confira se `claude`, `git` e `gh` estão **Conectados**. Para conectar, rode o comando indicado no cartão no Terminal e clique em **Verificar novamente**.
2. Em **Tarefas**, crie um workspace (nome e caminho completo da pasta do projeto, que precisa ser um repositório git) e digite um título na coluna **A fazer** (`Enter`).
3. Selecione a tarefa e clique em **Próximo passo** (ou tecle `A`): o Claude Code implementa numa cópia separada do projeto.
4. Quando o card for para **Revisão**, abra **Alterações da run**, **Rodar localmente** para testar e depois **Publicar PR** e **Fazer merge**. O botão principal sempre diz qual é o próximo passo.
5. Feche a janela para ocultá-la: o Harnessy continua avisando pela barra de menus (**H**) e por notificações. Use **H → Encerrar Harnessy** para sair.

Não coloque credenciais no chat: o armazenamento de rascunhos não é um cofre.

**Nunca usou?** Comece pelo [Guia do Harnessy](docs/user-guide.md): ele foi escrito para quem não é da área técnica, com uma página por assunto.

## O que está disponível

| Funcionalidade | Estado |
| --- | --- |
| Aplicativo desktop, navegação e painel de contexto | Disponível |
| Chat com rascunho local e sugestões | Disponível |
| Tarefas (quadro local com estados e histórico) | Disponível |
| GitHub, Codex e Claude Code | Estado real das sessões das CLIs `gh`, `codex` e `claude`; autenticação pelo terminal |
| Claude Code implementa em worktree próprio | Disponível: verificações rodadas pelo núcleo, falha estruturada, retomada de sessão, contraprova e ciclo com o próximo modelo |
| Revisão independente | Revisor escolhido na interface (Claude Code ou Codex, com modelo e "quando revisar"); por padrão a revisão é proporcional ao tamanho e ao risco da mudança (pula só documentação pequena e usa modelo menor em mudança pequena), como um staff engineer |
| Ações (Explicar, Dividir em passos, Nova Ação, Ações do repositório) | Disponível |
| Workspaces e tarefas locais em SQLite | Disponível (aba Tarefas locais) |
| Configuração `.harnessy/` versionada | Ações em `.harnessy/actions/` com confirmação por hash; perfil de verificações planejado |
| Ver as alterações, rodar localmente, publicar PR em draft, checks e merge | Disponível |
| Próximo passo por tarefa, filtro "Precisa de você", contador na barra de menus e notificações | Disponível |
| Automação por etapa (eu conduzo, assistido, delegado), QA e discovery | Planejados (a base da automação já existe no núcleo) |
| Deploy e Playwright | Planejados |

As configurações e workflows descritos na SPEC são requisitos futuros, não arquivos que você precise criar para abrir esta versão. Nenhum token ou Docker é necessário para explorar a fundação; as CLIs são opcionais e, quando presentes, sua sessão é apenas verificada.

## Gerar seu aplicativo

Com as dependências instaladas:

```bash
bun run desktop:build
```

Resultado: `src-tauri/target/release/bundle/macos/Harnessy.app`. Você pode copiá-lo pelo Finder para `~/Applications` ou `/Applications` para abrir pelo Spotlight. Encerre a cópia antiga antes de abrir uma nova.

Este é um build local: não há instalador DMG, atualização automática ou release pública assinada/notarizada nesta etapa.

## Documentação

| Preciso de… | Leia |
| --- | --- |
| Instalar, executar ou gerar um build | [Primeiros passos](docs/getting-started.md) |
| Aprender a usar, passo a passo, sem jargão | [Guia do Harnessy](docs/user-guide.md) |
| Entender uma palavra | [Glossário](docs/guia/glossario.md) |
| Saber o que é seguro e o que fica no meu Mac | [Segurança e privacidade](docs/guia/09-seguranca-e-privacidade.md) |
| Testar o ciclo completo no próprio Harnessy | [Roteiro de teste](docs/roteiro-dogfooding.md) |
| Resolver erro ao abrir, compilar ou salvar | [Solução de problemas](docs/troubleshooting.md) |
| Trabalhar no código e executar verificações | [Desenvolvimento](docs/development.md) |
| Usar e diagnosticar o harness de desenvolvimento | [Harness do repositório](docs/harness.md) |
| Entender o que foi implementado e validado | [Fundação técnica](docs/foundation.md) |
| Consultar arquitetura e critérios do produto | [Especificação](SPEC.md) |
| Acompanhar versões | [Changelog](CHANGELOG.md) |
| Ver o progresso por etapa e o que ficou pendente | [Progresso](docs/progress.md) |
| Consultar fontes sobre harness engineering | [Pesquisa](docs/research/harness-practices.md) |

## Versionamento

O produto segue SemVer, com versões independentes para a interface (`package.json`) e para o núcleo Rust (`src-tauri/Cargo.toml`). `package.json` fornece a versão exibida e distribuída; `bun run check` confere esse vínculo. Use `bun run test` para executar os testes Vitest e `bun run test:watch` durante o desenvolvimento. Consulte a política em [Desenvolvimento](docs/development.md#versionamento).
