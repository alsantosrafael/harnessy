//! Falhas estruturadas: transforma a saída de um check em [`Failure`]s, com
//! arquivo, linha, teste, esperado/obtido e trecho. Alimenta o prompt de
//! correção e o evento `failures` da run.
//!
//! Funções puras: não executam nada e não usam modelo. Saída de um runner não
//! reconhecido vira uma única falha com o final da saída.

use serde::Serialize;

/// Contexto máximo de cada falha reconhecida.
const EXCERPT_LIMIT: usize = 1024;
/// Final da saída usado quando nenhum runner é reconhecido.
const FALLBACK_LIMIT: usize = 4 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Failure {
    pub(super) check: String,
    pub(super) file: Option<String>,
    pub(super) line: Option<u32>,
    pub(super) test_name: Option<String>,
    pub(super) expected: Option<String>,
    pub(super) actual: Option<String>,
    pub(super) excerpt: String,
}

impl Failure {
    fn new(check: &str) -> Self {
        Self {
            check: check.to_owned(),
            file: None,
            line: None,
            test_name: None,
            expected: None,
            actual: None,
            excerpt: String::new(),
        }
    }
}

/// Converte a saída de um check em falhas. O comando escolhe o parser primeiro;
/// se ele não reconhecer nada (ex.: `bun run test`), os demais tentam pelo
/// conteúdo. Sem nenhum reconhecimento, devolve uma falha com o final da saída.
pub(super) fn parse(check: &str, program: &str, args: &[String], output: &str) -> Vec<Failure> {
    let text = strip_ansi(output);
    let lines: Vec<&str> = text.lines().collect();
    let preferred = runner_for(program, args);
    let others = ALL_RUNNERS
        .into_iter()
        .filter(|runner| Some(*runner) != preferred);
    for runner in preferred.into_iter().chain(others) {
        let found = runner.parse(check, &lines);
        if !found.is_empty() {
            return found;
        }
    }
    vec![Failure {
        excerpt: last_bytes(text.trim_end(), FALLBACK_LIMIT).to_owned(),
        ..Failure::new(check)
    }]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runner {
    Vitest,
    CargoTest,
    CargoCheck,
    Tsc,
    Pytest,
    Go,
}

const ALL_RUNNERS: [Runner; 6] = [
    Runner::Vitest,
    Runner::CargoTest,
    Runner::CargoCheck,
    Runner::Tsc,
    Runner::Pytest,
    Runner::Go,
];

impl Runner {
    fn parse(self, check: &str, lines: &[&str]) -> Vec<Failure> {
        match self {
            Self::Vitest => vitest(check, lines),
            Self::CargoTest => cargo_test(check, lines),
            Self::CargoCheck => cargo_check(check, lines),
            Self::Tsc => tsc(check, lines),
            Self::Pytest => pytest(check, lines),
            Self::Go => go(check, lines),
        }
    }
}

/// Runner indicado pelo próprio comando; `None` para scripts genéricos.
fn runner_for(program: &str, args: &[String]) -> Option<Runner> {
    let name = program.rsplit('/').next().unwrap_or(program);
    let first = args.first().map_or("", String::as_str);
    match name {
        "vitest" | "jest" => Some(Runner::Vitest),
        "cargo" => match first {
            "test" => Some(Runner::CargoTest),
            "check" | "clippy" | "build" => Some(Runner::CargoCheck),
            _ => None,
        },
        "tsc" => Some(Runner::Tsc),
        "pytest" => Some(Runner::Pytest),
        "python" | "python3" if args.iter().any(|arg| arg == "pytest") => Some(Runner::Pytest),
        "go" if first == "test" => Some(Runner::Go),
        _ => None,
    }
}

/// Bloco de uma falha do Vitest/Jest: linha de cabeçalho, com arquivo e teste.
struct Anchor {
    start: usize,
    file: Option<String>,
    name: Option<String>,
}

fn vitest(check: &str, lines: &[&str]) -> Vec<Failure> {
    let anchors = vitest_anchors(lines);
    if anchors.is_empty() {
        return vitest_marks(check, lines);
    }
    // O Vitest repete as falhas no resumo final; só a primeira ocorrência conta.
    let mut seen: Vec<(Option<String>, Option<String>)> = Vec::new();
    let mut failures = Vec::new();
    for (n, anchor) in anchors.iter().enumerate() {
        let key = (anchor.file.clone(), anchor.name.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let end = anchors.get(n + 1).map_or(lines.len(), |next| next.start);
        failures.push(vitest_failure(check, lines, anchor, end));
    }
    failures
}

fn vitest_anchors(lines: &[&str]) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    let mut file: Option<String> = None;
    for (start, line) in lines.iter().enumerate() {
        let text = line.trim();
        if let Some(rest) = text
            .strip_prefix("FAIL")
            .filter(|rest| rest.starts_with(char::is_whitespace))
        {
            // `FAIL arquivo > suíte > teste` (Vitest), `FAIL arquivo [ arquivo ]`
            // (erro de suíte) ou `FAIL arquivo` (Jest, que detalha em `●`).
            let rest = rest.trim();
            let found = rest
                .split(" > ")
                .next()
                .unwrap_or(rest)
                .split(" [")
                .next()
                .unwrap_or(rest)
                .trim();
            let found = (!found.is_empty()).then(|| found.to_owned());
            let name = rest
                .rsplit_once(" > ")
                .map(|(_, name)| name.trim().to_owned());
            if name.is_some() || rest.contains(" [") {
                anchors.push(Anchor {
                    start,
                    file: found.clone(),
                    name,
                });
            }
            file = found;
        } else if let Some(rest) = text.strip_prefix('●') {
            // `● suíte › teste` (Jest). `● Console` é saída de console, não falha.
            let rest = rest.trim();
            if rest == "Console" {
                continue;
            }
            let name = rest.rsplit(" › ").next().unwrap_or(rest).trim();
            anchors.push(Anchor {
                start,
                file: file.clone(),
                name: Some(name.to_owned()),
            });
        }
    }
    anchors
}

fn vitest_failure(check: &str, lines: &[&str], anchor: &Anchor, end: usize) -> Failure {
    let body = &lines[anchor.start + 1..end];
    let mut expected = None;
    let mut actual = None;
    if let Some((got, want)) = body
        .iter()
        .find(|line| line.contains("AssertionError"))
        .and_then(|line| chai_values(line))
    {
        actual = Some(got);
        expected = Some(want);
    }
    for line in body {
        let text = line.trim();
        if let Some(value) = text.strip_prefix("Expected:") {
            expected = Some(unquote(value));
        }
        if let Some(value) = text.strip_prefix("Received:") {
            actual = Some(unquote(value));
        }
    }
    expected = expected.or_else(|| diff_side(body, "- Expected", "- "));
    actual = actual.or_else(|| diff_side(body, "+ Received", "+ "));
    let (file, line) = body
        .iter()
        .find_map(|line| stack_location(line))
        .map_or((anchor.file.clone(), None), |(file, line)| {
            (Some(file), Some(line))
        });
    Failure {
        file,
        line,
        test_name: anchor.name.clone(),
        expected,
        actual,
        excerpt: excerpt(lines, anchor.start, end),
        ..Failure::new(check)
    }
}

/// Lado de um diff do Vitest (`- Expected` / `+ Received`). Só pega valores de
/// uma linha, logo após o cabeçalho; diffs de várias linhas saem truncados.
fn diff_side(body: &[&str], header: &str, prefix: &str) -> Option<String> {
    let at = body.iter().position(|line| line.trim() == header)?;
    body[at + 1..]
        .iter()
        .take(8)
        .find_map(|line| line.trim_start().strip_prefix(prefix))
        .map(unquote)
}

/// Saída verbosa sem blocos: uma falha por linha `✕ nome` (ou `×`).
fn vitest_marks(check: &str, lines: &[&str]) -> Vec<Failure> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            let text = line.trim();
            let name = text
                .strip_prefix('✕')
                .or_else(|| text.strip_prefix('×'))?
                .trim();
            let name = strip_duration(name);
            (!name.is_empty()).then(|| Failure {
                test_name: Some(name.to_owned()),
                excerpt: excerpt(lines, at, at + 1),
                ..Failure::new(check)
            })
        })
        .collect()
}

/// Remove o tempo final de uma linha verbosa (`teste 12ms`).
fn strip_duration(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((head, tail)) if tail.strip_suffix("ms").is_some_and(is_number) => head.trim_end(),
        _ => name,
    }
}

/// `expected 'a' to equal 'b'` (mensagem do chai, usada pelo Vitest).
fn chai_values(message: &str) -> Option<(String, String)> {
    let (_, rest) = message.split_once("expected ")?;
    [" to strictly equal ", " to equal "]
        .into_iter()
        .find_map(|marker| rest.split_once(marker))
        .map(|(got, want)| (unquote(got), unquote(want)))
}

/// Localização de pilha: `❯ arquivo:linha:coluna` ou `at ... (arquivo:linha:coluna)`.
/// Ignora `node_modules` e internals do Node.
fn stack_location(line: &str) -> Option<(String, u32)> {
    let text = line.trim();
    let target = if let Some(rest) = text.strip_prefix('❯') {
        rest.trim()
    } else {
        let rest = text.strip_prefix("at ")?;
        match rest.rfind('(') {
            Some(open) if rest.ends_with(')') => &rest[open + 1..rest.len() - 1],
            _ => rest,
        }
    };
    if target.contains("node_modules") || target.starts_with("node:") {
        return None;
    }
    split_location(target)
}

fn cargo_test(check: &str, lines: &[&str]) -> Vec<Failure> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            let name = line.strip_prefix("test ")?.strip_suffix(" ... FAILED")?;
            let (start, end) = stdout_block(lines, name).unwrap_or((at, at + 1));
            let block = &lines[start..end];
            let (file, line) = block.iter().find_map(|line| panic_location(line)).unzip();
            Some(Failure {
                file,
                line,
                test_name: Some(name.to_owned()),
                expected: labeled(block, "right:"),
                actual: labeled(block, "left:"),
                excerpt: excerpt(lines, start, end),
                ..Failure::new(check)
            })
        })
        .collect()
}

/// Bloco `---- nome stdout ----` até o próximo bloco ou o resumo.
fn stdout_block(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let header = format!("---- {name} stdout ----");
    let start = lines.iter().position(|line| line.trim() == header)?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            line.starts_with("---- ")
                || line.trim() == "failures:"
                || line.starts_with("test result:")
        })
        .map_or(lines.len(), |offset| start + 1 + offset);
    Some((start, end))
}

/// `panicked at arquivo:linha:coluna:` (formato atual) ou
/// `panicked at 'mensagem', arquivo:linha:coluna` (formato antigo).
fn panic_location(line: &str) -> Option<(String, u32)> {
    let (_, rest) = line.split_once("panicked at ")?;
    let rest = rest.trim_end_matches(':');
    let rest = rest
        .rsplit_once(", ")
        .map_or(rest, |(_, location)| location);
    split_location(rest)
}

/// `left:` e `right:` do `assert_eq!`, sem as aspas do Debug.
fn labeled(block: &[&str], label: &str) -> Option<String> {
    block
        .iter()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(unquote)
}

fn cargo_check(check: &str, lines: &[&str]) -> Vec<Failure> {
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&at| is_cargo_error(lines[at]))
        .collect();
    let mut failures = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        if is_cargo_summary(lines[start]) {
            continue;
        }
        let end = starts.get(n + 1).copied().unwrap_or(lines.len());
        let (file, line) = lines[start + 1..end]
            .iter()
            .find_map(|line| line.trim().strip_prefix("--> ").and_then(split_location))
            .unzip();
        failures.push(Failure {
            file,
            line,
            excerpt: excerpt(lines, start, end),
            ..Failure::new(check)
        });
    }
    failures
}

fn is_cargo_error(line: &str) -> bool {
    line.starts_with("error[") || line.starts_with("error: ")
}

/// Linhas-resumo do cargo, que não são falhas em si.
fn is_cargo_summary(line: &str) -> bool {
    line.starts_with("error: could not compile") || line.starts_with("error: aborting due to")
}

fn tsc(check: &str, lines: &[&str]) -> Vec<Failure> {
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&at| tsc_error(lines[at]).is_some())
        .collect();
    let mut failures = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(lines.len());
        let (file, line) = tsc_error(lines[start]).unzip();
        failures.push(Failure {
            file,
            line,
            excerpt: excerpt(lines, start, end),
            ..Failure::new(check)
        });
    }
    failures
}

/// `arquivo(linha,coluna): error TSxxxx: ...` ou `arquivo:linha:coluna - error TSxxxx`.
fn tsc_error(line: &str) -> Option<(String, u32)> {
    if let Some((head, _)) = line.split_once("): error TS") {
        let (file, coords) = head.rsplit_once('(')?;
        let (row, _) = coords.split_once(',')?;
        return Some((file.to_owned(), row.parse::<u32>().ok()?));
    }
    let (head, _) = line.split_once(" - error TS")?;
    split_location(head)
}

fn pytest(check: &str, lines: &[&str]) -> Vec<Failure> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            let spec = line.strip_prefix("FAILED ")?;
            let node = spec.split_once(" - ").map_or(spec, |(node, _)| node);
            let (path, name) = node.split_once("::").unwrap_or((node, ""));
            let function = name.rsplit("::").next().unwrap_or(name);
            let header = if function.is_empty() {
                None
            } else {
                lines
                    .iter()
                    .position(|line| is_pytest_header(line, function))
            };
            let (start, end) = match header {
                Some(start) => (start, pytest_block_end(lines, start)),
                None => (at, at + 1),
            };
            let block = &lines[start..end];
            let message = block
                .iter()
                .find_map(|line| line.trim_start().strip_prefix("E ").map(str::trim));
            let (actual, expected) = message.and_then(pytest_values).unzip();
            let (file, line) = block
                .iter()
                .find_map(|line| pytest_location(line))
                .map_or((Some(path.to_owned()), None), |(file, line)| {
                    (Some(file), Some(line))
                });
            Some(Failure {
                file,
                line,
                test_name: (!name.is_empty()).then(|| name.to_owned()),
                expected,
                actual,
                excerpt: excerpt(lines, start, end),
                ..Failure::new(check)
            })
        })
        .collect()
}

/// Cabeçalho `_____ nome _____` da seção de falhas do pytest.
fn is_pytest_header(line: &str, function: &str) -> bool {
    let text = line.trim();
    text.starts_with("___") && text.trim_matches('_').trim() == function
}

fn pytest_block_end(lines: &[&str], start: usize) -> usize {
    lines[start + 1..]
        .iter()
        .position(|line| line.starts_with("___") || line.starts_with("==="))
        .map_or(lines.len(), |offset| start + 1 + offset)
}

/// `arquivo.py:linha: Erro` (linha de localização do pytest).
fn pytest_location(line: &str) -> Option<(String, u32)> {
    let (head, _) = line.trim().split_once(": ")?;
    if head.contains(".py:") {
        split_location(head)
    } else {
        None
    }
}

/// `assert obtido == esperado` (o pytest mostra o lado esquerdo primeiro).
fn pytest_values(message: &str) -> Option<(String, String)> {
    let (_, rest) = message.split_once("assert ")?;
    let (actual, expected) = rest.split_once(" == ")?;
    Some((unquote(actual), unquote(expected)))
}

fn go(check: &str, lines: &[&str]) -> Vec<Failure> {
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&at| lines[at].trim_start().starts_with("--- FAIL: "))
        .collect();
    let mut failures = Vec::new();
    for &start in &starts {
        let name = lines[start]
            .trim_start()
            .trim_start_matches("--- FAIL: ")
            .split(' ')
            .next()
            .unwrap_or_default();
        // A mensagem do `t.Errorf` vem antes da linha `--- FAIL`: a janela começa na fronteira anterior.
        let from = lines[..start]
            .iter()
            .rposition(|line| is_go_boundary(line))
            .map_or(0, |at| at + 1);
        let end = lines[start + 1..]
            .iter()
            .position(|line| is_go_boundary(line))
            .map_or(lines.len(), |offset| start + 1 + offset);
        let (file, line, expected, actual) = match lines[from..end]
            .iter()
            .find_map(|line| go_location(line.trim()))
        {
            Some((file, line, message)) => {
                let (actual, expected) = go_values(&message).unzip();
                (Some(file), Some(line), expected, actual)
            }
            None => (None, None, None, None),
        };
        failures.push(Failure {
            file,
            line,
            test_name: Some(name.to_owned()),
            expected,
            actual,
            excerpt: excerpt(lines, from, end),
            ..Failure::new(check)
        });
    }
    failures
}

fn is_go_boundary(line: &str) -> bool {
    let text = line.trim_start();
    text.starts_with("--- ")
        || text.starts_with("=== ")
        || text.starts_with("FAIL")
        || text.starts_with("ok ")
}

/// `arquivo_test.go:linha: mensagem`, com a mensagem devolvida à parte.
fn go_location(text: &str) -> Option<(String, u32, String)> {
    let (head, message) = text.split_once(": ")?;
    let (file, line) = split_location(head)?;
    file.ends_with(".go")
        .then(|| (file, line, message.to_owned()))
}

/// `got X, want Y` (mensagem padrão do `testing`).
fn go_values(message: &str) -> Option<(String, String)> {
    let (_, rest) = message.split_once("got ")?;
    let (got, want) = rest.split_once(", want ")?;
    Some((unquote(got), unquote(want)))
}

fn excerpt(lines: &[&str], start: usize, end: usize) -> String {
    cut_at(lines[start..end].join("\n").trim(), EXCERPT_LIMIT).to_owned()
}

/// `arquivo:linha:coluna` (ou `arquivo:linha`) no fim de `text`. Exige um
/// arquivo com `.` ou `/`, para não confundir números soltos com localização.
fn split_location(text: &str) -> Option<(String, u32)> {
    let parts: Vec<&str> = text.trim().rsplitn(3, ':').collect();
    let (file, line) = match parts.as_slice() {
        [col, line, file] if is_number(col) && is_number(line) => (*file, *line),
        [line, file] if is_number(line) => (*file, *line),
        _ => return None,
    };
    let looks_like_path = file.contains('.') || file.contains('/');
    if !looks_like_path {
        return None;
    }
    Some((file.to_owned(), line.parse::<u32>().ok()?))
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// Tira aspas simples, duplas ou crases que envolvem um valor.
fn unquote(value: &str) -> String {
    let value = value.trim();
    let bytes = value.as_bytes();
    let wrapped = value.len() >= 2
        && matches!(bytes[0], b'"' | b'\'' | b'`')
        && bytes[value.len() - 1] == bytes[0];
    if wrapped {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

/// Remove sequências ANSI (`ESC[...letra`), que as CLIs emitem mesmo sem TTY.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for d in chars.by_ref() {
                if ('@'..='~').contains(&d) {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Prefixo de `text` com no máximo `limit` bytes, em fronteira de caractere.
pub(super) fn cut_at(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Sufixo de `text` com no máximo `limit` bytes, em fronteira de caractere.
fn last_bytes(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saída real do Vitest (formato v2/v3), com um teste falho.
    const VITEST_OUTPUT: &str = r#" ❯ src/lib/chatTask.test.ts (3 tests | 1 failed) 14ms
   ✓ separa o rótulo 2ms
   × divide título e descrição na primeira linha 6ms

 FAIL  src/lib/chatTask.test.ts > chatTask > divide título e descrição na primeira linha
AssertionError: expected 'Título\n' to equal 'Título'

- Expected
+ Received

- Título
+ Título
+

 ❯ src/lib/chatTask.test.ts:42:22
     40| it("divide título e descrição na primeira linha", () => {
     41|   const result = splitTitle("Título\nDescrição");
     42|   expect(result.title).toBe("Título");
       |                       ^
     43| });

 Test Files  1 failed (1)
      Tests  1 failed | 2 passed (3)
"#;

    /// Saída real do Jest, com um teste falho e um bloco de console (não é falha).
    const JEST_OUTPUT: &str = r#"PASS src/lib/other.test.ts
FAIL src/lib/chatTask.test.ts
  ● Console

    console.log
      separando

  ● chatTask › divide título e descrição na primeira linha

    expect(received).toBe(expected) // Object.is equality

    Expected: "Título"
    Received: "Título\n"

      41 |   const result = splitTitle("Título\nDescrição");
    > 42 |   expect(result.title).toBe("Título");
         |                        ^
      43 | });

      at Object.<anonymous> (src/lib/chatTask.test.ts:42:24)

Test Suites: 1 failed, 1 passed, 2 total
"#;

    /// Saída real de `cargo test` com um teste falho e panic no formato atual.
    const CARGO_TEST_OUTPUT: &str = r#"   Compiling harnessy v0.5.0 (/Users/rafael/Documents/harnessy/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.12s
     Running unittests src/lib.rs (target/debug/deps/harnessy_lib-3f2a)

running 3 tests
test flow::verify::tests::tail_keeps_the_end_on_a_char_boundary ... ok
test flow::verify::tests::run_all_reports ... FAILED
test flow::failure::tests::cut ... ok

failures:

---- flow::verify::tests::run_all_reports stdout ----
thread 'flow::verify::tests::run_all_reports' panicked at src/flow/verify.rs:201:9:
assertion `left == right` failed
  left: "Título\n"
 right: "Título"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    flow::verify::tests::run_all_reports

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
"#;

    /// Saída real de `cargo clippy -- -D warnings` com dois erros e o resumo.
    const CARGO_CHECK_OUTPUT: &str = r#"    Checking harnessy v0.5.0 (/Users/rafael/Documents/harnessy/src-tauri)
error[E0308]: mismatched types
  --> src/flow/verify.rs:88:12
   |
88 |     Ok(text)
   |     -- ^^^^ expected `String`, found `&str`
   |
help: try using a conversion method: `.to_string()`

error: this `if` has identical blocks
  --> src/flow/failure.rs:120:9
   |
   = note: `-D clippy::if-same-then-else` implied by `-D warnings`

error: could not compile `harnessy` (lib) due to 2 previous errors
"#;

    /// Saída real de `tsc --noEmit` com dois erros e o resumo.
    const TSC_OUTPUT: &str = r#"src/lib/chatTask.ts(42,12): error TS2322: Type 'string' is not assignable to type 'number'.
src/lib/chatTask.ts(57,3): error TS2554: Expected 1 arguments, but got 2.

Found 2 errors in the same file, starting at: src/lib/chatTask.ts:42
"#;

    /// Saída real do pytest com uma falha de asserção.
    const PYTEST_OUTPUT: &str = r#"============================= test session starts ==============================
collected 2 items

tests/test_chat.py .F                                                    [100%]

=================================== FAILURES ===================================
________________________________ test_divide _________________________________

    def test_divide():
>       assert divide("Título\nDescrição") == "Título"
E       AssertionError: assert 'Título\n' == 'Título'
E         - Título
E         + Título
E         +

tests/test_chat.py:12: AssertionError
=========================== short test summary info ============================
FAILED tests/test_chat.py::test_divide - AssertionError: assert 'Título\n' == 'Título'
========================= 1 failed, 1 passed in 0.05s ==========================
"#;

    /// Saída real de `go test` (sem -v): a mensagem vem antes de `--- FAIL`.
    const GO_OUTPUT: &str = r#"=== RUN   TestSplitTitle
    chat_test.go:42: got "Título\n", want "Título"
--- FAIL: TestSplitTitle (0.00s)
FAIL
FAIL	example.com/chat	0.214s
"#;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn vitest_output_gives_file_line_test_and_values() {
        let failures = parse("Testes", "vitest", &args(&["run"]), VITEST_OUTPUT);
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.check, "Testes");
        assert_eq!(failure.file.as_deref(), Some("src/lib/chatTask.test.ts"));
        assert_eq!(failure.line, Some(42));
        assert_eq!(
            failure.test_name.as_deref(),
            Some("divide título e descrição na primeira linha")
        );
        assert_eq!(failure.expected.as_deref(), Some("Título"));
        assert_eq!(failure.actual.as_deref(), Some("Título\\n"));
        assert!(failure.excerpt.contains("AssertionError"));
    }

    #[test]
    fn jest_output_ignores_console_blocks_and_reads_expected_received() {
        let failures = parse("Testes", "jest", &[], JEST_OUTPUT);
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.file.as_deref(), Some("src/lib/chatTask.test.ts"));
        assert_eq!(failure.line, Some(42));
        assert_eq!(
            failure.test_name.as_deref(),
            Some("divide título e descrição na primeira linha")
        );
        assert_eq!(failure.expected.as_deref(), Some("Título"));
        assert_eq!(failure.actual.as_deref(), Some("Título\\n"));
    }

    #[test]
    fn generic_script_is_recognised_by_content() {
        let failures = parse("Testes", "bun", &args(&["run", "test"]), VITEST_OUTPUT);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].line, Some(42));
    }

    #[test]
    fn cargo_test_output_gives_panic_location_and_left_right() {
        let failures = parse("Rust", "cargo", &args(&["test"]), CARGO_TEST_OUTPUT);
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.file.as_deref(), Some("src/flow/verify.rs"));
        assert_eq!(failure.line, Some(201));
        assert_eq!(
            failure.test_name.as_deref(),
            Some("flow::verify::tests::run_all_reports")
        );
        assert_eq!(failure.actual.as_deref(), Some("Título\\n"));
        assert_eq!(failure.expected.as_deref(), Some("Título"));
    }

    #[test]
    fn panic_location_accepts_the_older_message_format() {
        assert_eq!(
            panic_location("thread 'x' panicked at 'boom', src/lib.rs:10:5"),
            Some(("src/lib.rs".to_owned(), 10))
        );
    }

    #[test]
    fn cargo_check_output_gives_one_failure_per_error() {
        let failures = parse("Lint", "cargo", &args(&["clippy"]), CARGO_CHECK_OUTPUT);
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].file.as_deref(), Some("src/flow/verify.rs"));
        assert_eq!(failures[0].line, Some(88));
        assert!(failures[0].excerpt.contains("mismatched types"));
        assert_eq!(failures[1].file.as_deref(), Some("src/flow/failure.rs"));
        assert_eq!(failures[1].line, Some(120));
        assert_eq!(failures[1].test_name, None);
    }

    #[test]
    fn tsc_output_gives_file_and_line_of_each_error() {
        let failures = parse("Tipos", "tsc", &args(&["--noEmit"]), TSC_OUTPUT);
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].file.as_deref(), Some("src/lib/chatTask.ts"));
        assert_eq!(failures[0].line, Some(42));
        assert!(failures[0].excerpt.contains("TS2322"));
        assert_eq!(failures[1].line, Some(57));
    }

    #[test]
    fn tsc_pretty_form_is_understood() {
        assert_eq!(
            tsc_error("src/a.ts:3:7 - error TS2304: Cannot find name 'x'."),
            Some(("src/a.ts".to_owned(), 3))
        );
    }

    #[test]
    fn pytest_output_gives_file_line_and_assert_values() {
        let failures = parse("Testes", "pytest", &[], PYTEST_OUTPUT);
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.file.as_deref(), Some("tests/test_chat.py"));
        assert_eq!(failure.line, Some(12));
        assert_eq!(failure.test_name.as_deref(), Some("test_divide"));
        assert_eq!(failure.actual.as_deref(), Some("Título\\n"));
        assert_eq!(failure.expected.as_deref(), Some("Título"));
    }

    #[test]
    fn go_output_gives_file_line_and_got_want() {
        let failures = parse("Testes", "go", &args(&["test", "./..."]), GO_OUTPUT);
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.file.as_deref(), Some("chat_test.go"));
        assert_eq!(failure.line, Some(42));
        assert_eq!(failure.test_name.as_deref(), Some("TestSplitTitle"));
        assert_eq!(failure.actual.as_deref(), Some("Título\\n"));
        assert_eq!(failure.expected.as_deref(), Some("Título"));
    }

    #[test]
    fn unknown_output_falls_back_to_one_failure_with_the_text() {
        let failures = parse("Build", "make", &[], "linha 1\nerro estranho\n");
        assert_eq!(failures.len(), 1);
        let failure = &failures[0];
        assert_eq!(failure.check, "Build");
        assert_eq!(failure.file, None);
        assert_eq!(failure.line, None);
        assert_eq!(failure.test_name, None);
        assert_eq!(failure.excerpt, "linha 1\nerro estranho");
    }

    #[test]
    fn fallback_keeps_the_last_4_kb_on_a_char_boundary() {
        let output = "é".repeat(3000);
        let failures = parse("Build", "make", &[], &output);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].excerpt.len() <= FALLBACK_LIMIT);
        assert!(failures[0].excerpt.ends_with('é'));
    }

    #[test]
    fn recognised_excerpt_is_capped_at_1_kb() {
        let output = format!(
            "FAIL  a.test.ts > t\nAssertionError: x\n{}\n ❯ a.test.ts:3:1",
            "x".repeat(3000)
        );
        let failures = parse("Testes", "vitest", &[], &output);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].excerpt.len() <= EXCERPT_LIMIT);
        assert_eq!(failures[0].line, Some(3));
    }

    #[test]
    fn ansi_codes_are_removed_before_parsing() {
        assert_eq!(strip_ansi("\u{1b}[31m×\u{1b}[39m teste"), "× teste");
        let output = "\u{1b}[31m FAIL \u{1b}[39m src/a.test.ts > x > falha\nAssertionError: boom\n ❯ src/a.test.ts:7:1";
        let failures = parse("Testes", "vitest", &[], output);
        assert_eq!(failures[0].file.as_deref(), Some("src/a.test.ts"));
        assert_eq!(failures[0].line, Some(7));
    }
}
