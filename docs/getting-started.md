# Primeiros passos

[← README](../README.md) · [Guia de uso](user-guide.md) · [Solução de problemas](troubleshooting.md)

## Escolha como executar

| Opção | Quando usar | Requisitos |
| --- | --- | --- |
| Aplicativo já compilado | Explorar a interface sem ferramentas de desenvolvimento | Mac compatível com a arquitetura do build |
| Desenvolvimento desktop | Alterar o código e acompanhar atualizações | Bun, Rust e ferramentas Apple; Node 22+ para Vitest |
| Prévia web | Trabalhar apenas na interface React | Bun e navegador |

A validação atual é em macOS Apple Silicon. Windows, Linux e Mac Intel ainda não têm builds validados. Um build Apple Silicon não equivale a um build universal.

## 1. Se o aplicativo já foi gerado

No Finder, abra a pasta do projeto e navegue até:

```text
src-tauri/target/release/bundle/macos/Harnessy.app
```

Dê dois cliques no aplicativo. Se preferir o terminal, execute a partir da raiz do projeto:

```bash
open src-tauri/target/release/bundle/macos/Harnessy.app
```

Você pode copiar o `.app` pelo Finder para uma pasta de aplicativos. A cópia funciona sem o repositório e sem Vite; ela só é atualizada quando você substitui o build manualmente.

Se o arquivo não existir, siga as etapas de preparação e build abaixo. Não há download de release pública configurado neste repositório.

## 2. Preparar as ferramentas

Estas etapas são necessárias para compilar ou desenvolver, não para abrir um aplicativo pronto.

### Bun e runtime de testes

Instale **Bun 1.4.3 ou superior** seguindo o [site oficial](https://bun.sh/docs/installation). O projeto usa Bun para dependências e scripts, Vite para desenvolvimento/build e Vitest para testes. Não use `bun test`: o comando correto é **`bun run test`**, que seleciona Vitest.

O runner Vitest está configurado para ambiente Node; mantenha **Node.js 22+** disponível pelo [site oficial](https://nodejs.org/) ou seu gerenciador habitual. Isso não muda o gerenciador de pacotes para npm.

Confira no terminal:

```bash
bun --version
node --version
```

### Rust

Instale Rust estável seguindo [rustup.rs](https://rustup.rs/). Depois, abra um novo terminal e confira:

```bash
rustc --version
cargo --version
```

O build da fundação foi validado com Rust 1.95. Dependências travadas podem exigir um compilador recente; use o toolchain estável atualizado. O campo de versão mínima do crate não representa uma matriz de compatibilidade já testada.

### Ferramentas Apple

Confira se as ferramentas de desenvolvimento estão disponíveis:

```bash
xcode-select -p
```

Se não estiverem instaladas, solicite a instalação pelo macOS:

```bash
xcode-select --install
```

Conclua o diálogo da Apple antes de continuar. Não é necessário instalar uma CLI Tauri global: o projeto já a declara como dependência de desenvolvimento.

## 3. Instalar as dependências do projeto

Abra o terminal na pasta `harnessy` — a mesma que contém `package.json` e `src-tauri/` — e execute:

```bash
bun install --frozen-lockfile
```

Esse comando usa as versões de `bun.lock`. A primeira compilação Rust baixa as dependências descritas em `src-tauri/Cargo.lock`; é necessário acesso à internet para esses downloads iniciais.

Não é preciso criar `.env`, configurar provider, instalar Codex, conectar GitHub ou iniciar Docker nesta etapa.

## 4. Abrir no modo de desenvolvimento

```bash
bun run start
```

O comando inicia o Vite em `http://127.0.0.1:1420` e abre a janela Tauri. Mantenha o terminal aberto. Mudanças na interface são atualizadas pelo servidor de desenvolvimento; alterações Rust podem exigir recompilação.

Na interface desktop, o cabeçalho deve indicar **Núcleo local conectado** depois de receber os metadados do Rust. Isso indica comunicação IPC, não uma conexão ao Codex.

Para encerrar a sessão, use **H → Encerrar Harnessy** e `Ctrl+C` no terminal se o processo de desenvolvimento continuar ativo.

### Apenas a interface no navegador

```bash
bun run dev
```

Abra `http://127.0.0.1:1420`. Nesse modo, **Prévia da interface** é o estado esperado: não há janela Tauri ou núcleo Rust. Use `Ctrl+C` para parar o servidor.

Não execute `bun run dev` e `bun run start` simultaneamente: ambos tentam iniciar o Vite na mesma porta.

## 5. Gerar um build para uso diário

Encerre uma sessão de desenvolvimento ativa e execute:

```bash
bun run check
bun run test
bun run desktop:build
```

O segundo comando compila o frontend e o núcleo e gera o `.app`. O caminho aparece no final do terminal:

```text
src-tauri/target/release/bundle/macos/Harnessy.app
```

Abra pelo Finder ou pelo comando da etapa 1. O bundle atual é local, sem assinatura/notarização de distribuição. Se o macOS bloquear a abertura, consulte [os procedimentos de diagnóstico](troubleshooting.md#o-macos-bloqueou-o-aplicativo).

## 6. Confirmar que está pronto

- A janela abre e mostra a versão em **Sobre esta fundação**.
- Tarefas e Chat alternam normalmente.
- **Preparar tarefa** leva o cursor ao campo de texto.
- O rascunho mostra o status de salvamento.
- **Conexões** mostra o estado real das sessões do GitHub, Codex e Claude Code.
- O menu **H** permite reabrir a janela depois de fechá-la.

Continue no [guia de uso](user-guide.md). Se algo divergir, consulte [solução de problemas](troubleshooting.md).
