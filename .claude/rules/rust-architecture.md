---
paths:
  - "src-tauri/**"
---

# Regras do núcleo Rust (`src-tauri`)

Mapa e justificativas em `docs/architecture-rust.md`. Estas regras valem para qualquer edição em `src-tauri/src`.

## Estrutura

1. `lib.rs` só registra módulos e monta o `tauri::Builder`. Nenhuma lógica, struct ou comando é definido nele.
2. Um módulo por domínio (`workspace`, `task`, `connections`, `app`) e um por infraestrutura compartilhada (`db`, `error`). Novo conceito de produto = novo diretório `src/<dominio>/`, nunca uma função solta em módulo existente.
3. Dentro de um domínio: `mod.rs` contém modelos e regras; `commands.rs` contém apenas adaptadores `#[tauri::command]`. Submódulos adicionais (`probe.rs`, `process.rs`, `interpret.rs`) separam dados estáticos, efeitos colaterais e lógica pura.
4. Módulos de domínio não se importam entre si. Compartilhamento passa por `db` ou `error`; se dois domínios precisarem do mesmo tipo, ele vira módulo próprio.

## Comandos IPC

5. Um comando faz três coisas: obtém dependências do `State`, chama uma função do domínio, devolve o resultado. Sem validação, SQL ou `format!` de mensagens em `commands.rs`.
6. Toda função de domínio que toca o banco recebe `&rusqlite::Connection`, nunca `State` nem `Database`. Isso mantém o domínio testável sem Tauri.
7. Registrar o comando em `lib.rs` e, se a superfície IPC mudar, atualizar `src/lib/desktop.ts`, `docs/development.md` e `capabilities/main.json` na mesma mudança.
8. Tipos serializados usam `#[serde(rename_all = "camelCase")]`; entradas usam também `deny_unknown_fields`. Nomes de campo são o contrato com o frontend e não mudam sem versionar.

## Erros

9. Erro de domínio é `crate::error::AppError`; funções devolvem `crate::error::Result<T>`. Não criar `Result<T, String>` nem `Box<dyn Error>` fora de `app::setup`.
10. Cada variante carrega a frase exibida ao usuário, em português, sem detalhes técnicos. Erros de biblioteca entram via `#[from]` e são traduzidos pela mensagem da variante, nunca propagados em texto.
11. Mensagens de validação são `AppError::Validation(&'static str)`. Mensagens dinâmicas exigem variante própria com `#[error("...{0}...")]`.
12. `unwrap`/`expect` são proibidos fora de testes e da inicialização em `lib.rs`.

## Persistência

13. Esquema só muda adicionando uma entrada ao final de `MIGRATIONS` em `db.rs`. Nunca editar uma migração já publicada; `user_version` é a única fonte da versão do esquema.
14. Limites de tamanho e constantes de validação ficam no topo do módulo de domínio como `const`, não como literais nas funções.
15. Nomes de colunas em `SELECT` são explícitos (sem `*`), mapeados por uma função `from_row` no tipo.

## Segurança e processos externos

16. Executar binários externos só em `connections::process`, com `stdin` fechado, tempo limite, saída limitada e argumentos fixos vindos de `probe.rs`. Nenhum argumento vem da WebView.
17. Credenciais, tokens e saída bruta de CLIs nunca entram em um tipo `Serialize`. A interface recebe estado, frase curta e, no máximo, nome de conta e caminho do binário.
18. Novo acesso privilegiado (filesystem, rede, shell) exige comando dedicado, permissão mínima em `capabilities/main.json` e nota em `docs/development.md`.

## Testes e verificação

19. Lógica pura e acesso ao banco são testados no próprio módulo (`#[cfg(test)]`), usando `db::test_support::TempDatabase`. Não criar arquivos temporários à mão.
20. Parsers de saída externa (`interpret.rs`) recebem texto fixo em teste; nunca executar a CLI real em testes.
21. Antes de concluir: `bun run verify:rust` sem avisos e `cargo test` verde. Mudança em IPC, janela ou bandeja exige ainda o smoke desktop de `docs/development.md`.

## Estilo

22. Comentário de módulo (`//!`) explica responsabilidade e fronteiras; comentário de item (`///`) só onde o nome não basta. Texto em português, como o restante do projeto.
23. Preferir `impl` no tipo (`Task::from_row`, `Verdict::new`) a funções soltas quando a operação pertence ao tipo. Preferir `&str` e `&Path` em parâmetros, `String`/`PathBuf` só quando há posse.
24. Nada de `pub` por conveniência: o que o frontend consome é `pub` via `commands.rs`; o restante é privado ou `pub(crate)`.
