import assert from "node:assert/strict";
import { test } from "vitest";
import { agentOf, agents, defaultChoice, modelLabel, normalizeChoice } from "../src/lib/agents.ts";
import { splitDraft } from "../src/lib/chatTask.ts";

test("the default is Claude Code with Sonnet, and every executable agent lists its default model", () => {
  assert.deepEqual(defaultChoice(), { runtime: "claude", model: "sonnet", reviewer: "claude" });
  for (const agent of agents.filter((item) => !item.unavailable)) {
    assert.ok(agent.models.some((model) => model.id === agent.defaultModel));
  }
});

test("a stored choice is kept only if it is executable and the model is known", () => {
  assert.deepEqual(normalizeChoice({ runtime: "claude", model: "opus" }), {
    runtime: "claude",
    model: "opus",
    reviewer: "claude",
  });
  assert.deepEqual(normalizeChoice({ runtime: "claude", model: "gpt-5" }), defaultChoice());
  assert.deepEqual(normalizeChoice({ runtime: "codex", model: "x" }), defaultChoice());
  assert.deepEqual(normalizeChoice({ runtime: "outro" }), defaultChoice());
  assert.deepEqual(normalizeChoice(null), defaultChoice());
  assert.deepEqual(normalizeChoice("lixo"), defaultChoice());
});

test("the reviewer is kept when valid, even if the implementer choice is discarded", () => {
  assert.equal(
    normalizeChoice({ runtime: "claude", model: "opus", reviewer: "codex" }).reviewer,
    "codex",
  );
  assert.deepEqual(normalizeChoice({ runtime: "codex", reviewer: "codex" }), {
    ...defaultChoice(),
    reviewer: "codex",
  });
  assert.equal(normalizeChoice({ reviewer: "gpt" }).reviewer, "claude");
});

test("Codex is listed as implementer but not selectable: it only reviews", () => {
  assert.ok(agentOf("codex")?.unavailable);
  assert.equal(agentOf("claude")?.unavailable, undefined);
  assert.equal(modelLabel({ runtime: "claude", model: "fable", reviewer: "claude" }), "Fable");
});

test("the first line becomes the title and the rest the description", () => {
  assert.deepEqual(splitDraft("  Corrigir o login\n\nO botão não responde.\nVer o console.  "), {
    title: "Corrigir o login",
    description: "O botão não responde.\nVer o console.",
  });
  assert.deepEqual(splitDraft("Só uma linha"), { title: "Só uma linha", description: "" });
});

test("a long first line is cut for the title and kept whole in the description", () => {
  const long = "a".repeat(150);
  const split = splitDraft(`${long}\nresto`);
  assert.equal(split.title.length, 100);
  assert.ok(split.title.endsWith("…"));
  assert.equal(split.description, `${long}\nresto`);
});
