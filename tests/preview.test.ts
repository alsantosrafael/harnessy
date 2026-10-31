import assert from "node:assert/strict";
import { test } from "vitest";
import type { PreviewInfo } from "../src/lib/desktop.ts";
import { appendLog, isLive, previewLabel, upsertPreview } from "../src/lib/preview.ts";

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
