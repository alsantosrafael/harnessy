import assert from "node:assert/strict";
import { test } from "vitest";
import type { PreviewInfo } from "../src/lib/desktop.ts";
import {
  appendLog,
  hintFor,
  isLive,
  logKind,
  previewLabel,
  upsertPreview,
} from "../src/lib/preview.ts";

const info = (extra: Partial<PreviewInfo>): PreviewInfo => ({
  runId: 1,
  state: "running",
  url: "http://localhost:5173",
  exitCode: null,
  startedAt: 0,
  ...extra,
});

test("labels say the state in words and mark failures with the exit code", () => {
  assert.equal(previewLabel(info({ state: "starting" })).text, "Subindo…");
  assert.equal(previewLabel(info({})).text, "No ar");
  assert.deepEqual(previewLabel(info({ state: "failed", exitCode: 3 })), {
    text: "Caiu (código 3)",
    tone: "critical",
  });
  assert.equal(previewLabel(info({ state: "stopped" })).text, "Parado");
  assert.equal(previewLabel(null).text, "Desligado");
});

test("only starting and running count as live", () => {
  assert.equal(isLive(info({ state: "starting" })), true);
  assert.equal(isLive(info({})), true);
  assert.equal(isLive(info({ state: "failed" })), false);
  assert.equal(isLive(null), false);
});

test("the log keeps the latest lines and states replace by run", () => {
  assert.deepEqual(appendLog(["a", "b"], "c", 2), ["b", "c"]);
  const next = upsertPreview({ 1: info({ state: "starting" }), 2: info({ runId: 2 }) }, info({}));
  assert.equal(next[1].state, "running");
  assert.equal(next[2].runId, 2);
});

test("a port clash on a failed environment gets a hint about the {port} marker", () => {
  const failed = info({ state: "failed", exitCode: 1 });
  const log = ["$ vite", "error when starting dev server:", "Error: Port 1420 is already in use"];
  assert.match(String(hintFor(failed, log)), /\{port\}/);
  assert.match(
    String(hintFor(failed, ["Error: listen EADDRINUSE: address already in use :::3000"])),
    /ocupada/,
  );
  assert.equal(hintFor(failed, ["erro qualquer"]), null);
  assert.equal(hintFor(info({}), log), null, "só quando o ambiente caiu");
  assert.equal(hintFor(null, log), null);
});

test("log lines are classified for the terminal, with errors winning over everything", () => {
  assert.equal(logKind("$ bunx --bun vite --host 127.0.0.1"), "command");
  assert.equal(logKind("A cópia desta run não existe mais; recriando a partir da branch…"), "note");
  assert.equal(logKind("Instalando as dependências…"), "note");
  assert.equal(logKind("Error: Port 1420 is already in use"), "error");
  assert.equal(logKind('error: script "dev" exited with code 1'), "error");
  assert.equal(logKind("error when starting dev server: ready in 3 ms"), "error", "erro vence ok");
  assert.equal(logKind("warning: unused import"), "warn");
  assert.equal(logKind("  VITE v6.4.5  ready in 82 ms"), "ok");
  assert.equal(logKind("  ➜  Local:   http://localhost:5173/"), "info");
  assert.equal(logKind("    at onError (/x/vite.js:25336:22)"), "dim");
  assert.equal(logKind("alguma coisa qualquer"), "plain");
  assert.equal(logKind("   "), "plain");
});
