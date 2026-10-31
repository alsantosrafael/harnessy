import type { Finding } from "./feed";

export type LineKind = "add" | "del" | "ctx";

export interface DiffLine {
  kind: LineKind;
  /** Número no arquivo antigo; `null` em linhas adicionadas. */
  oldNo: number | null;
  /** Número no arquivo novo; `null` em linhas removidas. */
  newNo: number | null;
  text: string;
}

export interface Hunk {
  header: string;
  lines: DiffLine[];
}

const HUNK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** Lê os hunks de um patch unificado, numerando as linhas dos dois lados. */
export function parsePatch(patch: string): Hunk[] {
  const hunks: Hunk[] = [];
  let current: Hunk | null = null;
  let oldNo = 0;
  let newNo = 0;
  for (const raw of patch.split("\n")) {
    const header = HUNK.exec(raw);
    if (header) {
      current = { header: raw, lines: [] };
      hunks.push(current);
      oldNo = Number(header[1]);
      newNo = Number(header[2]);
      continue;
    }
    if (!current || raw === "" || raw.startsWith("\\")) continue;
    const sign = raw[0];
    const text = raw.slice(1);
    if (sign === "+") current.lines.push({ kind: "add", oldNo: null, newNo: newNo++, text });
    else if (sign === "-") current.lines.push({ kind: "del", oldNo: oldNo++, newNo: null, text });
    else current.lines.push({ kind: "ctx", oldNo: oldNo++, newNo: newNo++, text });
  }
  return hunks;
}

/** Achados da revisão de um arquivo, indexados pela linha do arquivo novo. */
export function findingsByLine(findings: Finding[], path: string): Map<number, Finding[]> {
  const map = new Map<number, Finding[]>();
  for (const finding of findings) {
    if (finding.arquivo !== path) continue;
    map.set(finding.linha, [...(map.get(finding.linha) ?? []), finding]);
  }
  return map;
}

/** Resumo para o cabeçalho: "3 arquivos · +40 −12". */
export function diffSummary(files: number, additions: number, deletions: number): string {
  const noun = files === 1 ? "arquivo" : "arquivos";
  return `${files} ${noun} · +${additions} −${deletions}`;
}
