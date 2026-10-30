//! Filtro de texto antes de gravar eventos: remove padrões de token e limita o tamanho.

const MAX_EVENT_BYTES: usize = 4000;
const PREFIXES: [&str; 8] = [
    "sk-ant-",
    "sk-",
    "ghp_",
    "gho_",
    "ghs_",
    "github_pat_",
    "AKIA",
    "xox",
];
const MIN_TOKEN_LEN: usize = 20;
const MASK: &str = "[redigido]";

/// Substitui palavras que parecem credenciais e trunca o texto.
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_EVENT_BYTES + 16));
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if looks_like_secret(word) {
            out.push_str(MASK);
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for ch in text.chars() {
        if ch.is_whitespace() || matches!(ch, '"' | '\'' | '=' | ':' | ',' | '`') {
            flush(&mut word, &mut out);
            out.push(ch);
        } else {
            word.push(ch);
        }
    }
    flush(&mut word, &mut out);
    if text.contains("PRIVATE KEY-----") {
        return "[redigido: chave privada]".to_owned();
    }
    truncate(out)
}

fn looks_like_secret(word: &str) -> bool {
    word.len() >= MIN_TOKEN_LEN && PREFIXES.iter().any(|prefix| word.starts_with(prefix))
}

fn truncate(mut text: String) -> String {
    if text.len() > MAX_EVENT_BYTES {
        let mut end = MAX_EVENT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push('…');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_token_like_words() {
        let cleaned =
            clean("export KEY=sk-ant-api03-abcdefghijklmnop and ghp_0123456789abcdefghij ok");
        assert!(!cleaned.contains("sk-ant"));
        assert!(!cleaned.contains("ghp_0123"));
        assert!(cleaned.contains("ok"));
        assert_eq!(clean("sk-curto"), "sk-curto");
    }

    #[test]
    fn drops_private_keys_and_truncates() {
        assert_eq!(
            clean("-----BEGIN OPENSSH PRIVATE KEY-----\nabc"),
            "[redigido: chave privada]"
        );
        let long = "ç".repeat(5000);
        let cleaned = clean(&long);
        assert!(cleaned.len() <= MAX_EVENT_BYTES + 4);
        assert!(cleaned.ends_with('…'));
    }
}
