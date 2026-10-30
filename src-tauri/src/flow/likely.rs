//! Arquivos prováveis de uma Tarefa: termos do título e dos critérios casados
//! com nomes de arquivo e com o conteúdo rastreado (`git grep`). Só caminhos,
//! nunca conteúdo; nenhuma chamada de modelo. Serve a qualquer modelo: menos
//! turnos de exploração antes da primeira edição.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{process, run::worktree};

/// Caminhos entregues ao agente.
const MAX_FILES: usize = 10;
/// Termos buscados por conteúdo (cada um é um `git grep`).
const MAX_TERMS: usize = 8;
const MIN_TERM_CHARS: usize = 4;
/// Termo que casa com mais arquivos que isto não distingue nada.
const MAX_HITS_PER_TERM: usize = 200;
/// Casamento no nome do arquivo vale mais que no conteúdo.
const NAME_WEIGHT: usize = 3;

const STOPWORDS: &[&str] = &[
    "para", "como", "quando", "deve", "devem", "pode", "podem", "sobre", "entre", "mais", "menos",
    "cada", "todo", "toda", "todos", "todas", "esse", "essa", "este", "esta", "isso", "isto",
    "aquele", "aquela", "onde", "qual", "quais", "sem", "com", "uma", "umas", "uns", "pelo",
    "pela", "pelos", "pelas", "depois", "antes", "ainda", "também", "apenas", "fazer", "faz",
    "ficar", "fica", "seja", "sejam", "está", "estão", "tela", "quero", "precisa", "deveria",
    "with", "from", "that", "this", "when", "should", "must", "into", "have", "make", "only",
    "then", "than", "them", "they", "will", "each", "other", "some", "what", "which",
];

/// Termos distintos da tarefa: palavras de 4+ letras sem stopwords, e
/// identificadores (`camelCase`, `snake_case`, `kebab-case`) inteiros e em partes.
pub fn terms(title: &str, criteria: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let text = format!("{title}\n{criteria}");
    for raw in text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.')) {
        let raw = raw.trim_matches(|c: char| c == '.' || c == '-' || c == '_');
        let mut candidates = vec![raw.to_owned()];
        if is_identifier(raw) {
            candidates.extend(split_identifier(raw));
        }
        for candidate in candidates {
            let lower = candidate.to_lowercase();
            if lower.chars().count() < MIN_TERM_CHARS
                || stop.contains(lower.as_str())
                || lower.chars().all(|c| c.is_ascii_digit())
            {
                continue;
            }
            if seen.insert(lower.clone()) {
                out.push(lower);
            }
        }
    }
    out
}

fn is_identifier(word: &str) -> bool {
    word.contains('_')
        || word.contains('-')
        || word.contains('.')
        || word.chars().skip(1).any(char::is_uppercase)
}

fn split_identifier(word: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for c in word.chars() {
        if c == '_' || c == '-' || c == '.' {
            parts.push(std::mem::take(&mut current));
        } else if c.is_uppercase() && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
            current.push(c);
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts.into_iter().filter(|part| !part.is_empty()).collect()
}

/// Ordena caminhos por pontuação: termo no nome do arquivo vale [`NAME_WEIGHT`],
/// termo no conteúdo vale 1. Empate: caminho mais curto primeiro.
pub fn rank(
    files: &[String],
    terms: &[String],
    content_hits: &[(String, Vec<String>)],
) -> Vec<String> {
    let mut score: HashMap<&str, usize> = HashMap::new();
    for file in files {
        let name = file.to_lowercase();
        let hits = terms
            .iter()
            .filter(|term| name.contains(term.as_str()))
            .count();
        if hits > 0 {
            *score.entry(file).or_default() += hits * NAME_WEIGHT;
        }
    }
    for (_, hits) in content_hits {
        if hits.len() > MAX_HITS_PER_TERM {
            continue;
        }
        for file in hits {
            if let Some(known) = files.iter().find(|candidate| *candidate == file) {
                *score.entry(known).or_default() += 1;
            }
        }
    }
    let mut ranked: Vec<(&str, usize)> = score
        .into_iter()
        .filter(|(file, _)| !is_noise(file))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.len().cmp(&b.0.len()))
            .then(a.0.cmp(b.0))
    });
    ranked
        .into_iter()
        .take(MAX_FILES)
        .map(|(file, _)| file.to_owned())
        .collect()
}

/// Lockfiles e arquivos gerados não ajudam a decidir onde mexer.
fn is_noise(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    matches!(
        name,
        "bun.lock"
            | "bun.lockb"
            | "Cargo.lock"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "uv.lock"
            | "poetry.lock"
            | "go.sum"
            | "CHANGELOG.md"
    ) || name.ends_with(".min.js")
        || name.ends_with(".snap")
}

/// Arquivos prováveis do repositório, um por linha. Vazio quando nada casou ou
/// quando o git não está disponível (a variável vira "(nenhum encontrado)").
pub fn find(repo: &Path, title: &str, criteria: &str) -> String {
    let dirs = process::search_dirs();
    let Some(git) = process::find_binary("git", &dirs) else {
        return String::new();
    };
    let Ok(files) = worktree::ls_files(&git, &dirs, repo) else {
        return String::new();
    };
    let terms = terms(title, criteria);
    let content_hits: Vec<(String, Vec<String>)> = terms
        .iter()
        .take(MAX_TERMS)
        .filter_map(|term| {
            worktree::grep_files(&git, &dirs, repo, term)
                .ok()
                .map(|hits| (term.clone(), hits))
        })
        .collect();
    rank(&files, &terms, &content_hits).join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn terms_drop_short_words_and_stopwords_and_split_identifiers() {
        let found = terms(
            "Corrigir o parseDuration quando a entrada tem espaços",
            "- O run_cycles deve gravar o modelo",
        );
        for expected in [
            "corrigir",
            "parseduration",
            "parse",
            "duration",
            "entrada",
            "espaços",
        ] {
            assert!(
                found.contains(&expected.to_owned()),
                "falta {expected}: {found:?}"
            );
        }
        for expected in ["run_cycles", "cycles", "gravar", "modelo"] {
            assert!(found.contains(&expected.to_owned()), "falta {expected}");
        }
        for dropped in ["o", "a", "tem", "quando", "deve", "run"] {
            assert!(!found.contains(&dropped.to_owned()), "sobrou {dropped}");
        }
    }

    #[test]
    fn rank_prefers_name_matches_ignores_noise_and_caps_the_list() {
        let files = owned(&[
            "src/lib/duration.ts",
            "tests/duration.test.ts",
            "src/lib/other.ts",
            "bun.lock",
            "src/components/Run.tsx",
        ]);
        let terms = owned(&["duration", "parse"]);
        let hits = vec![
            (
                "duration".to_owned(),
                owned(&["src/lib/other.ts", "bun.lock"]),
            ),
            ("parse".to_owned(), owned(&["src/lib/duration.ts"])),
        ];
        let ranked = rank(&files, &terms, &hits);
        assert_eq!(
            ranked,
            [
                "src/lib/duration.ts",
                "tests/duration.test.ts",
                "src/lib/other.ts"
            ]
        );

        let many: Vec<String> = (0..30).map(|i| format!("src/duration{i}.ts")).collect();
        assert_eq!(rank(&many, &terms, &[]).len(), MAX_FILES);
        assert!(rank(&files, &owned(&["inexistente"]), &[]).is_empty());
    }

    #[test]
    fn terms_matching_too_many_files_do_not_count() {
        let files: Vec<String> = (0..300).map(|i| format!("f{i}.rs")).collect();
        let hits = vec![("comum".to_owned(), files.clone())];
        assert!(rank(&files, &owned(&["comum"]), &hits).is_empty());
    }

    #[test]
    fn find_reads_a_real_repository() {
        let dirs = process::search_dirs();
        let Some(git) = process::find_binary("git", &dirs) else {
            return;
        };
        let root = std::env::temp_dir().join(format!("harnessy-likely-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/slugify.ts"), "export const slugify = 1;").unwrap();
        std::fs::write(
            root.join("src/uses.ts"),
            "import { slugify } from './slugify';",
        )
        .unwrap();
        std::fs::write(root.join("README.md"), "nada aqui").unwrap();
        for args in [vec!["init", "-q"], vec!["add", "-A"]] {
            let status = std::process::Command::new(&git)
                .args(&args)
                .current_dir(&root)
                .status()
                .unwrap();
            assert!(status.success());
        }
        let found = find(&root, "Corrigir slugify com acentos", "");
        assert_eq!(found, "src/slugify.ts\nsrc/uses.ts");
        let _ = std::fs::remove_dir_all(&root);
    }
}
