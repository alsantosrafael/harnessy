import assert from "node:assert/strict";
import { test } from "vitest";
import { buildFeed, parseReview } from "../src/lib/feed.ts";

const event = (seq: number, kind: string, text: string) => ({ runId: 1, seq, kind, text });

test("consecutive tool calls collapse into one group; messages and status stay apart", () => {
  const feed = buildFeed([
    event(1, "status", "Criando worktree"),
    event(2, "tool", "Read a.ts"),
    event(3, "tool", "Edit a.ts"),
    event(4, "assistant", "Pronto."),
    event(5, "tool", "Bash bun test"),
  ]);
  assert.deepEqual(
    feed.map((item) => item.kind),
    ["status", "tools", "message", "tools"],
  );
  assert.deepEqual(feed[1], { kind: "tools", seq: 2, lines: ["Read a.ts", "Edit a.ts"] });
});

test("technical logs are grouped and the steps proposal is left to its own screen", () => {
  const feed = buildFeed([
    event(1, "stderr", "ruído"),
    event(2, "log", "linha"),
    event(3, "steps", "[]"),
    event(4, "result", "success · 3 turnos"),
  ]);
  assert.deepEqual(
    feed.map((item) => item.kind),
    ["logs", "result"],
  );
});

test("the review event becomes a card with who reviewed and the findings", () => {
  const text = JSON.stringify({
    reviewer: "codex",
    verdict: "request_changes",
    blocking: [{ arquivo: "a.ts", linha: 3, severidade: "bloqueia", texto: "Falta tratar null." }],
    suggestions: [],
    discarded: 2,
  });
  const [item] = buildFeed([event(1, "review", text)]);
  assert.equal(item.kind, "review");
  assert.equal(parseReview(text)?.reviewer, "codex");
  assert.equal(parseReview(text)?.blocking[0].linha, 3);
  assert.equal(parseReview(text)?.discarded, 2);
});

test("a malformed review event is dropped instead of breaking the feed", () => {
  assert.equal(parseReview("não é json"), null);
  assert.equal(parseReview('{"sem":"veredito"}'), null);
  assert.deepEqual(buildFeed([event(1, "review", "{")]), []);
  assert.deepEqual(parseReview('{"verdict":"approve","blocking":[{"x":1}]}')?.blocking, []);
});
