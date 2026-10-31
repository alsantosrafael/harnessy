//! Alterações de uma run para o humano ler: o diff `base..branch` separado por arquivo.
//!
//! Só interpretação de texto. Quem obtém o diff é `worktree::diff_range`; aqui ele
//! vira arquivos com contagem de linhas, dentro de limites para a tela não travar.

use serde::Serialize;

const MAX_FILES: usize = 200;
const MAX_PATCH_BYTES: usize = 60 * 1024;
const MAX_TOTAL_BYTES: usize = 400 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    /// `modified`, `added`, `deleted` ou `renamed`.
    pub status: &'static str,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
    /// Hunks do arquivo (a partir do primeiro `@@`); vazio para binário.
    pub patch: String,
    /// O patch passou do limite e foi cortado.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDiff {
    pub files: Vec<FileDiff>,
    pub additions: u32,
    pub deletions: u32,
    /// Havia mais arquivos ou bytes do que a tela mostra.
    pub truncated: bool,
}

/// Separa um diff unificado em arquivos. Limites de arquivos e bytes mantêm a tela leve.
pub fn parse(text: &str) -> RunDiff {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut truncated = false;
    let mut total = 0usize;
    for block in text.split("\ndiff --git ").enumerate().map(|(i, block)| {
        if i == 0 {
            block.strip_prefix("diff --git ").unwrap_or(block)
        } else {
            block
        }
    }) {
        if block.trim().is_empty() {
            continue;
        }
        if files.len() >= MAX_FILES || total >= MAX_TOTAL_BYTES {
            truncated = true;
            break;
        }
        let file = parse_file(block);
        total += file.patch.len();
        files.push(file);
    }
    let additions = files.iter().map(|file| file.additions).sum();
    let deletions = files.iter().map(|file| file.deletions).sum();
    RunDiff {
        files,
        additions,
        deletions,
        truncated,
    }
}

fn parse_file(block: &str) -> FileDiff {
    let mut lines = block.lines();
    let header = lines.next().unwrap_or("");
    // "a/x b/x": o caminho novo é o que vem depois de " b/".
    let path = header
        .rsplit_once(" b/")
        .map(|(_, new)| new.to_owned())
        .unwrap_or_else(|| header.to_owned());
    let mut status = "modified";
    let mut binary = false;
    let mut patch = String::new();
    let (mut additions, mut deletions) = (0u32, 0u32);
    let mut in_hunks = false;
    let mut truncated = false;
    for line in lines {
        if !in_hunks {
            if line.starts_with("new file mode") {
                status = "added";
            } else if line.starts_with("deleted file mode") {
                status = "deleted";
            } else if line.starts_with("rename from") || line.starts_with("rename to") {
                status = "renamed";
            } else if line.starts_with("Binary files") || line.starts_with("GIT binary patch") {
                binary = true;
            } else if line.starts_with("@@") {
                in_hunks = true;
            }
        }
        if !in_hunks {
            continue;
        }
        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
        if patch.len() + line.len() + 1 > MAX_PATCH_BYTES {
            truncated = true;
            continue;
        }
        patch.push_str(line);
        patch.push('\n');
    }
    FileDiff {
        path,
        status,
        additions,
        deletions,
        binary,
        patch,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "diff --git a/src/a.ts b/src/a.ts\nindex 1..2 100644\n--- a/src/a.ts\n+++ b/src/a.ts\n@@ -1,2 +1,3 @@\n keep\n-old\n+new\n+more\ndiff --git a/img.png b/img.png\nnew file mode 100644\nBinary files /dev/null and b/img.png differ\ndiff --git a/old.md b/new.md\nsimilarity index 90%\nrename from old.md\nrename to new.md\n@@ -1 +1 @@\n-a\n+b\n";

    #[test]
    fn splits_files_and_counts_lines_without_counting_headers() {
        let diff = parse(DIFF);
        assert_eq!(diff.files.len(), 3);
        let first = &diff.files[0];
        assert_eq!(
            (first.path.as_str(), first.status),
            ("src/a.ts", "modified")
        );
        assert_eq!((first.additions, first.deletions), (2, 1));
        assert!(first.patch.starts_with("@@ -1,2 +1,3 @@"));
        assert!(!first.patch.contains("+++"));
        assert_eq!((diff.additions, diff.deletions), (3, 2));
    }

    #[test]
    fn detects_binary_added_and_renamed_files() {
        let diff = parse(DIFF);
        assert!(diff.files[1].binary && diff.files[1].patch.is_empty());
        assert_eq!(diff.files[1].status, "added");
        assert_eq!(
            (diff.files[2].path.as_str(), diff.files[2].status),
            ("new.md", "renamed")
        );
    }

    #[test]
    fn long_patches_and_many_files_are_cut_not_dropped_silently() {
        let line = format!("+{}\n", "x".repeat(100));
        let big = format!("diff --git a/b b/b\n@@ -0,0 +1 @@\n{}", line.repeat(1_000));
        let diff = parse(&big);
        assert!(diff.files[0].truncated);
        assert!(diff.files[0].patch.len() <= MAX_PATCH_BYTES);
        assert_eq!(
            diff.files[0].additions, 1_000,
            "a contagem usa o diff inteiro"
        );

        let many: String = (0..250)
            .map(|i| format!("diff --git a/f{i} b/f{i}\n@@ -0,0 +1 @@\n+x\n"))
            .collect();
        let diff = parse(&many);
        assert_eq!(diff.files.len(), MAX_FILES);
        assert!(diff.truncated);
    }

    #[test]
    fn empty_input_is_an_empty_diff() {
        assert_eq!(parse("").files.len(), 0);
    }
}
