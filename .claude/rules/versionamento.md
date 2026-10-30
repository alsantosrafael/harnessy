# Regra de versionamento

Front e back do Harnessy são versionados **de forma independente**:

- **Front:** `package.json` → `version`. É a versão do produto exibida e distribuída; o Tauri lê esse valor.
- **Back:** `src-tauri/Cargo.toml` → `version` do pacote `harnessy`. É a versão do núcleo Rust.

Os números não precisam coincidir. A política SemVer e as justificativas estão em `docs/development.md#versionamento`; esta rule define **o que fazer em cada mudança**.

## Obrigatório a cada modificação

Antes de considerar qualquer mudança concluída, avalie a versão de **front** e a de **back** e aplique o bump que couber. Não há negociação: não pergunte se deve versionar, não deixe para uma release e não pule o passo porque a mudança parece pequena. Se a mudança não afeta um dos lados, esse lado permanece igual.

Afeta o **front** qualquer mudança em `src/`, em estilos, em textos da interface, em `package.json` quando altera o produto, ou em `tauri.conf.json`.

Afeta o **back** qualquer mudança em `src-tauri/`, incluindo IPC, persistência, lifecycle, dependências Rust e a configuração de build nativo.

Mudanças que afetam os dois lados (ex.: um comando IPC novo com tela que o consome) recebem bump nos dois.

## Nível do bump

- **Funcionalidade** (`feat`): MINOR.
- **Correção** (`fix`): PATCH.
- **Refactor** (`refactor`), inclusive interno, sem mudança de comportamento: PATCH.
- **Desempenho** (`perf`) que altera comportamento observável: PATCH.
- **Mudança incompatível** (`breaking`, incluindo contratos IPC, schema de dados ou configuração, contrato de agente): durante 0.x, MINOR com migração explícita; após 1.0, MAJOR.
- **Só docs, testes sem efeito no produto, CI ou ferramentas do harness** (`chore`): sem bump.

Se uma mudança misturar tipos, use o maior nível aplicável ao lado afetado.

## Como aplicar

1. Para cada lado afetado, calcule a nova versão a partir do valor atual do arquivo correspondente.
2. Atualize o arquivo do lado afetado:
   - front → `package.json` (`"version"`);
   - back → `src-tauri/Cargo.toml` (`version`), seguido de `cargo check --manifest-path src-tauri/Cargo.toml` para reconciliar `src-tauri/Cargo.lock`.
3. Registre a mudança em `CHANGELOG.md` em uma seção por lado afetado, no topo, no formato `## Front [x.y.z] — AAAA-MM-DD` ou `## Back [x.y.z] — AAAA-MM-DD`, com itens em português. Marque mudanças incompatíveis com `**BREAKING:**`.
4. Rode `bun run check` (inclui a checagem de versão do front) e, para mudanças nativas, `bun run verify:rust`.

## Limites

- Não edite lockfiles à mão. `bun.lock` muda só via `bun add`/`bun install`; `Cargo.lock` muda via `cargo`.
- Não reutilize uma versão já registrada no `CHANGELOG.md`.
- Não crie outro campo de versão na UI, em schemas ou em configs. Schema de dados e contratos de agentes têm versões próprias, conforme `docs/development.md`.
- Não crie tag, release ou push sem autorização explícita. Versionar é alterar arquivos; publicar é outra decisão.
