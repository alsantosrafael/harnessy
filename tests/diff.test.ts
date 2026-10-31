import assert from "node:assert/strict";
import { test } from "vitest";
import { diffSummary, findingsByLine, parsePatch } from "../src/lib/diff.ts";

const PATCH =
  "@@ -1,3 +1,4 @@\n keep\n-old\n+new\n+more\n tail\n@@ -20 +21,2 @@\n+a\n+b\n\\ No newline at end of file\n";

test("hunks number both sides and mark additions, removals and context", () => {
  const hunks = parsePatch(PATCH);
  assert.equal(hunks.length, 2);
  const lines = hunks[0].lines;
  assert.deepEqual(
    lines.map((l) => [l.kind, l.oldNo, l.newNo]),
    [
      ["ctx", 1, 1],
      ["del", 2, null],
      ["add", null, 2],
      ["add", null, 3],
      ["ctx", 3, 4],
    ],
  );
  assert.equal(lines[1].text, "old");
  assert.equal(hunks[1].lines[0].newNo, 21, "o segundo hunk recomeça pela numeração do cabeçalho");
  assert.equal(hunks[1].lines.length, 2, "a linha 'No newline' não vira código");
});

test("review findings attach to the new-file line of their own file only", () => {
  const findings = [
    { arquivo: "a.ts", linha: 3, severidade: "bloqueia", texto: "x" },
    { arquivo: "a.ts", linha: 3, severidade: "sugere", texto: "y" },
    { arquivo: "b.ts", linha: 3, severidade: "sugere", texto: "z" },
  ];
  const map = findingsByLine(findings, "a.ts");
  assert.equal(map.get(3)?.length, 2);
  assert.equal(findingsByLine(findings, "c.ts").size, 0);
});

test("the summary pluralizes and shows signs", () => {
  assert.equal(diffSummary(1, 4, 0), "1 arquivo · +4 −0");
  assert.equal(diffSummary(3, 40, 12), "3 arquivos · +40 −12");
});
