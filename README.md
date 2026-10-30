# Harnessy

**Um espaço local para organizar trabalho, conversar com agentes e acompanhar entregas.**

Aplicativo desktop feito com **Tauri 2, Rust, React e TypeScript**, usando **Bun, Vite e Vitest** no frontend. Primeira plataforma: **macOS**, validado inicialmente em Apple Silicon.

> **0.2.0 · Em desenvolvimento.** Crie tarefas locais, mova os cards entre A fazer, Em andamento, Revisão e Concluído (com histórico) e mande o Claude Code implementar em um worktree próprio. O card anda sozinho e as runs presas aparecem como bloqueadas. Push e PR ainda são manuais.

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

1. Em **Visão geral → Tarefas**, crie um workspace e digite o título de uma tarefa na raia **A fazer** (`Enter`). Mova o card pela seta, com `]`/`[` ou arrastando.
2. Clique em **Preparar tarefa** para abrir o Chat com o campo em foco.
3. Escreva sua ideia. O rascunho é salvo neste dispositivo; sugestões acrescentam texto sem apagar o conteúdo existente.
4. Abra **Conexões** para verificar as sessões das CLIs `gh`, `codex` e `claude`. Para autenticar, execute o comando indicado no terminal e verifique novamente.
5. Feche a janela para ocultá-la. Use o menu **H**, na barra superior do macOS, para **Abrir Harnessy** ou **Encerrar Harnessy**.

No chat, **Iniciar** cria a tarefa e começa a run com o agente e o modelo que você escolher (padrão: Claude Code com Sonnet). Não coloque credenciais no chat: o armazenamento de rascunhos não é um cofre.

[Guia completo de uso e atalhos →](docs/user-guide.md)

## O que está disponível

| Funcionalidade | Estado |
| --- | --- |
| Aplicativo desktop, navegação e painel de contexto | Disponível |
| Chat com rascunho local e sugestões | Disponível |
| Tarefas (quadro local com estados e histórico) | Disponível |
| GitHub, Codex e Claude Code | Estado real das sessões das CLIs `gh`, `codex` e `claude`; autenticação pelo terminal |
| Claude Code implementa em worktree próprio | Disponível: verificações rodadas pelo núcleo, falha estruturada, retomada de sessão, contraprova e ciclo com o próximo modelo |
| Revisão independente | Revisor Claude (um modelo acima) quando há sinal de risco; revisão pelo Codex planejada |
| Ações (Explicar, Dividir em passos, Nova Ação, Ações do repositório) | Disponível |
| Workspaces e tarefas locais em SQLite | Disponível (aba Tarefas locais) |
| Configuração `.harnessy/` versionada | Ações em `.harnessy/actions/` com confirmação por hash; perfil de verificações planejado |
| Deploy, Playwright e fluxo de revisão/correção | Planejados |

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
| Entender telas, atalhos e rascunhos | [Guia de uso](docs/user-guide.md) |
| Resolver erro ao abrir, compilar ou salvar | [Solução de problemas](docs/troubleshooting.md) |
| Trabalhar no código e executar verificações | [Desenvolvimento](docs/development.md) |
| Usar e diagnosticar o harness de desenvolvimento | [Harness do repositório](docs/harness.md) |
| Entender o que foi implementado e validado | [Fundação técnica](docs/foundation.md) |
| Consultar arquitetura e critérios do produto | [Especificação](SPEC.md) |
| Acompanhar versões | [Changelog](CHANGELOG.md) |
| Ver o progresso por etapa e o que ficou pendente | [Progresso](docs/progress.md) |
| Consultar fontes sobre harness engineering | [Pesquisa](docs/research/harness-practices.md) |

## Versionamento

O produto começa em **0.1.0**, com SemVer desde o início. `package.json` fornece a versão exibida e distribuída; `bun run check` confere esse vínculo. Use `bun run test` para executar os testes Vitest e `bun run test:watch` durante o desenvolvimento. Consulte a política em [Desenvolvimento](docs/development.md#versionamento).
