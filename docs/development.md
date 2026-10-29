# Desenvolvimento

[← README](../README.md) · [Instalação](getting-started.md) · [Fundação e validações](foundation.md)

## Ciclo diário

Todos os comandos partem da raiz do projeto:

```bash
bun install --frozen-lockfile
bun run start
```

`bun install --frozen-lockfile` é necessário no primeiro setup e quando as dependências travadas mudam, não a cada alteração de código. O Tauri inicia o Vite automaticamente. Para alterar dependências, use `bun add`/`bun add -d` e versione `bun.lock`. Não mantenha lockfiles de gerenciadores diferentes.

## Comandos disponíveis

| Comando | Resultado |
| --- | --- |
| `bun run start` | Desktop Tauri em desenvolvimento com Vite. |
| `bun run dev` | Frontend no navegador em `127.0.0.1:1420`, sem núcleo Rust. |
| `bun run check` | TypeScript e vínculo de versão do produto. |
| `bun run build` | TypeScript e build frontend em `dist/`. |
| `bun run check:rust` | Compilação de verificação Rust, sem bundle. |
| `bun run desktop:build` | Frontend, binário de produção e bundle macOS `.app`. |
| `bun run tauri info` | Diagnóstico do ambiente Tauri. |
| `bun run test` | Testes de regressão com Vitest. |
| `bun run test:watch` | Vitest em modo interativo. |

Verificações adicionais:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Vitest tem configuração própria em `vitest.config.ts`, com ambiente Node e casos em `tests/`. A suíte inicial verifica o contrato de sessão de rascunhos: compatibilidade, corrupção, falha de escrita, retry, sugestões e debounce. Não use `bun test` (runner nativo do Bun); use `bun run test` para executar Vitest. Node 22+ continua necessário para o runner/worker. As sessões Playwright registradas em [foundation.md](foundation.md) complementam a suíte, mas não substituem automação desktop.

## Estrutura

```text
src/
  App.tsx                  # Navegação, layout e telas iniciais
  components/              # Board, Chat, Conexões e Contexto
  lib/desktop.ts           # Interface tipada para metadados Rust
  lib/drafts.ts            # Sessão de rascunho, recuperação e persistência
  styles/tokens.css        # Primitivos, semânticos e componentes
  styles/app.css           # Estilos da fundação
  styles/refinements.css   # Refinamentos de UX e adaptação
src-tauri/
  src/lib.rs               # Lifecycle, bandeja e comando app_info
  src/main.rs              # Entrada do executável
  capabilities/main.json   # Permissões da janela
  tauri.conf.json          # Janela, CSP, build e bundle
  icons/                   # Ícones gerados
scripts/check-version.mjs  # Validação da fonte de versão
tests/drafts.test.ts        # Regressões executadas pelo Vitest
vitest.config.ts            # Ambiente e descoberta dos testes
bun.lock                    # Dependências frontend travadas
docs/                      # Uso, desenvolvimento e pesquisa
```

## Regras da fundação

- Rust controla operações privilegiadas; React não recebe shell ou acesso genérico a arquivos.
- O único comando IPC implementado é `app_info`; ele retorna versão, plataforma e disponibilidade de execução, atualmente `false`.
- Não apresentar integração como funcional sem implementação real. Estados vazios e desabilitados são intencionais.
- A sessão do rascunho vive durante toda a aplicação e só o chat assina suas atualizações; evitar atualização do shell inteiro a cada tecla. Navegar não destrói texto não persistido.
- Usar tokens para aparência e preservar teclado, foco e movimento reduzido.
- Não usar localStorage para credenciais. O rascunho existente é dado de interface, não banco de domínio.
- A fundação não contém sandbox de agentes: restringir capabilities da WebView não comprova isolamento de futuros processos Codex.

## Versionamento

### Fonte de verdade

`package.json.version` fornece a versão do produto. `src-tauri/tauri.conf.json` referencia `../package.json`; Vite importa o mesmo valor para a interface. `bun run check` valida o vínculo e o formato básico da versão.

O crate Rust usa `0.0.0` como versão interna e não é distribuído separadamente. Não altere essa versão para simular uma release do produto.

### Política SemVer

- **PATCH:** correção compatível.
- **MINOR:** funcionalidade compatível.
- **MAJOR, após 1.0:** mudança incompatível nos contratos públicos.
- **Durante 0.x:** mudança incompatível exige ao menos nova MINOR e migração explícita.
- Pré-releases usam nomes como `0.2.0-alpha.1` e `0.2.0-beta.1`.

Schema de dados/configuração e contratos de agentes/workflows terão versões próprias quando implementados. O rascunho atual usa schema `1`.

### Preparar uma versão local

1. Escolha a versão conforme a política e atualize `CHANGELOG.md`.
2. Atualize `package.json.version` e execute `bun install` para reconciliar o lockfile quando necessário. A release deve manter uma única versão de produto; não crie outro campo de versão na UI.
3. Execute os checks TypeScript/Rust e gere o bundle.
4. Confira a versão exibida no app e valide os fluxos afetados.

Esses comandos não criam uma release pública. Commit, tag, assinatura, notarização, publicação e updater ainda não têm pipeline configurado. Não existe repositório remoto oficial informado nesta etapa.

## Ícones

O desenho de origem está em `app-icon.svg`. Para regenerar os arquivos com a CLI local:

```bash
bun run tauri icon app-icon.svg
```

A CLI pode gerar também tamanhos para plataformas não utilizadas; o bundle desktop usa a lista de ícones de `tauri.conf.json`.

## Smoke check antes de entregar

1. Abrir o desktop e confirmar a versão e os metadados locais.
2. Alternar telas, abas e painel de contexto por mouse e teclado.
3. Preparar tarefa, escrever, inserir sugestão e verificar que o texto foi preservado.
4. Aguardar salvamento e confirmar o rascunho após reabertura no mesmo contexto.
5. Abrir Sobre, fechar com Escape e conferir retorno do foco.
6. Conferir que conexões e envio continuam indisponíveis enquanto não implementados.
7. Fechar janela, reabrir pela bandeja e encerrar explicitamente.

IPC e lifecycle precisam de verificação no Tauri real. Navegador/WebKit ajuda a validar a interface, mas não comprova a integração desktop. Para requisitos e próximos gates, consulte [SPEC.md](../SPEC.md).
