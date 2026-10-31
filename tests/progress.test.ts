import assert from "node:assert/strict";
import { test } from "vitest";
import type { Run, RunEvent } from "../src/lib/desktop.ts";
import { activityOf, describeTool, elapsedLabel, progressOf } from "../src/lib/progress.ts";

const event = (seq: number, kind: string, text: string): RunEvent => ({
  runId: 1,
  seq,
  kind,
  text,
});

test("tool events become short sentences in Portuguese", () => {
  assert.equal(
    describeTool("Edit: /Users/x/proj/docs/guia/glossario.md"),
    "Editando guia/glossario.md",
  );
  assert.equal(describeTool("Read: AGENTS.md"), "Lendo AGENTS.md");
  assert.equal(describeTool("Bash: bun run check"), "Rodando bun run check");
  assert.equal(describeTool("Bash"), "Rodando");
  assert.equal(describeTool("WebFetch"), "WebFetch", "ferramenta desconhecida fica como veio");
  assert.ok(describeTool(`Bash: ${"x".repeat(200)}`).length < 80);
});

test("the current activity is the latest tool or stage, ignoring chatter", () => {
  const events = [
    event(1, "user", "pedido"),
    event(2, "tool", "Read: a.md"),
    event(3, "message", "pensando"),
    event(4, "status", "Rodando as verificações do projeto"),
  ];
  assert.equal(activityOf(events), "Rodando as verificações do projeto");
  assert.equal(activityOf(events.slice(0, 3)), "Lendo a.md");
  assert.equal(activityOf([event(1, "message", "oi")]), null);
  assert.equal(activityOf(undefined), null);
});

test("elapsed time is m:ss and never negative", () => {
  assert.equal(elapsedLabel(100, 142_000), "0:42");
  assert.equal(elapsedLabel(0, 125_000), "2:05");
  assert.equal(elapsedLabel(500, 100_000), "0:00");
});

test("progress names the phase from the run state", () => {
  const run = (state: string) => ({ state, startedAt: 10 }) as Run;
  assert.equal(progressOf(run("preparing"), [], 20_000).phase, "Preparando a cópia do projeto");
  assert.equal(
    progressOf(run("implementing"), [event(1, "tool", "Edit: a")], 20_000).activity,
    "Editando a",
  );
});
