# Harnessy

**Um espaço local para organizar trabalho, conversar com agentes e acompanhar entregas.**

Aplicativo desktop feito com **Tauri 2, Rust, React e TypeScript**, usando **Bun, Vite e Vitest** no frontend. Primeira plataforma: **macOS**, validado inicialmente em Apple Silicon.

> **0.1.0 · Fundação em desenvolvimento.** Você já pode abrir o aplicativo, navegar por Board e Chat e preparar um rascunho. GitHub e Codex ainda não estão conectados; esta versão não executa agentes.

## Comece aqui

### Já tem o build neste projeto?

Na pasta raiz do projeto, execute:

```bash
open src-tauri/target/release/bundle/macos/Harnessy.app
```

Ou abra `Harnessy.app` pelo Finder nesse mesmo caminho. O aplicativo compilado não precisa de um terminal ou servidor Vite em execução.

### Quer executar pelo código-fonte?

Você precisa de **Bun 1.4.3+, Rust estável e Xcode Command Line Tools**. Para o runner Vitest, mantenha também Node.js 22+ disponível. Se ainda não preparou essas ferramentas, siga o [guia de instalação](docs/getting-started.md).

No terminal, entre na pasta do projeto e execute:

```bash
bun install --frozen-lockfile
bun run start
```

O comando abre a janela desktop e inicia automaticamente o frontend de desenvolvimento. A primeira compilação Rust pode demorar alguns minutos; acompanhe o terminal.

## Primeiro passeio

1. Em **Visão geral → Board**, conheça a estrutura do workspace. O board fica vazio até existir integração com o GitHub.
2. Clique em **Preparar tarefa** para abrir o Chat com o campo em foco.
3. Escreva sua ideia. O rascunho é salvo neste dispositivo; sugestões acrescentam texto sem apagar o conteúdo existente.
4. Abra **Conexões** para consultar o estado das integrações. Os botões estão desabilitados nesta etapa.
5. Feche a janela para ocultá-la. Use o menu **H**, na barra superior do macOS, para **Abrir Harnessy** ou **Encerrar Harnessy**.

O botão **Iniciar** ainda não envia o rascunho. Não coloque credenciais no chat: o armazenamento de rascunhos não é um cofre.

[Guia completo de uso e atalhos →](docs/user-guide.md)

## O que está disponível

| Funcionalidade | Estado |
| --- | --- |
| Aplicativo desktop, navegação e painel de contexto | Disponível |
| Chat com rascunho local e sugestões | Disponível |
| Board | Estrutura visual, sem cards reais |
| GitHub e Codex | Integrações planejadas, botões desabilitados |
| Agentes, worktrees e execução de tarefas | Planejados |
| SQLite, configuração `.harnessy/` e workspaces persistidos | Planejados |
| Deploy, Playwright e fluxo de revisão/correção | Planejados |

As configurações e workflows descritos na SPEC são requisitos futuros, não arquivos que você precise criar para abrir esta versão. Nenhum token, conta GitHub, Codex ou Docker é necessário para explorar a fundação.

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
| Entender telas, atalhos e rascunhos | [Guia de uso](docs/user-guide.md) |
| Resolver erro ao abrir, compilar ou salvar | [Solução de problemas](docs/troubleshooting.md) |
| Trabalhar no código e executar verificações | [Desenvolvimento](docs/development.md) |
| Usar e diagnosticar o harness de desenvolvimento | [Harness do repositório](docs/harness.md) |
| Entender o que foi implementado e validado | [Fundação técnica](docs/foundation.md) |
| Consultar arquitetura e critérios do produto | [Especificação](SPEC.md) |
| Acompanhar versões | [Changelog](CHANGELOG.md) |
| Consultar fontes sobre harness engineering | [Pesquisa](docs/research/harness-practices.md) |

## Versionamento

O produto começa em **0.1.0**, com SemVer desde o início. `package.json` fornece a versão exibida e distribuída; `bun run check` confere esse vínculo. Use `bun run test` para executar os testes Vitest e `bun run test:watch` durante o desenvolvimento. Consulte a política em [Desenvolvimento](docs/development.md#versionamento).
