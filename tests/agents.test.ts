import assert from "node:assert/strict";
import { test } from "vitest";
import {
  agentOf,
  agents,
  choiceSummary,
  defaultChoice,
  modelName,
  normalizeChoice,
} from "../src/lib/agents.ts";
import { splitDraft } from "../src/lib/chatTask.ts";

test("the default is Claude Code with Sonnet, reviewed by Claude Code when there is risk", () => {
  assert.deepEqual(defaultChoice(), {
    runtime: "claude",
    model: "sonnet",
    reviewer: "claude",
    reviewerModel: "",
    reviewWhen: "auto",
  });
  for (const agent of agents) {
    assert.ok(agent.models.some((model) => model.id === agent.defaultModel));
  }
});

test("a stored choice keeps known models and falls back to the agent default otherwise", () => {
  assert.deepEqual(normalizeChoice({ runtime: "claude", model: "opus" }), {
    ...defaultChoice(),
    model: "opus",
  });
  assert.equal(normalizeChoice({ runtime: "claude", model: "gpt-5" }).model, "sonnet");
  assert.deepEqual(normalizeChoice(null), defaultChoice());
  assert.deepEqual(normalizeChoice("lixo"), defaultChoice());
  assert.equal(normalizeChoice({ runtime: "outro" }).runtime, "claude");
});

test("Codex can implement and review, with the default or a typed model id", () => {
  assert.ok(agentOf("codex")?.customModel);
  const codex = normalizeChoice({
    runtime: "codex",
    model: "gpt-5.1-codex",
    reviewer: "codex",
    reviewerModel: "",
    reviewWhen: "always",
  });
  assert.deepEqual(codex, {
    runtime: "codex",
    model: "gpt-5.1-codex",
    reviewer: "codex",
    reviewerModel: "codex",
    reviewWhen: "always",
  });
  assert.equal(normalizeChoice({ runtime: "codex", model: "--sandbox x" }).model, "codex");
  assert.equal(normalizeChoice({ runtime: "codex", model: "" }).model, "codex");
});

test("the Claude reviewer may stay automatic; unknown policies fall back to auto", () => {
  assert.equal(normalizeChoice({ reviewer: "claude", reviewerModel: "" }).reviewerModel, "");
  assert.equal(
    normalizeChoice({ reviewer: "claude", reviewerModel: "opus" }).reviewerModel,
    "opus",
  );
  assert.equal(normalizeChoice({ reviewWhen: "talvez" }).reviewWhen, "auto");
});

test("the summary says who implements, who reviews and when", () => {
  assert.equal(
    choiceSummary(defaultChoice()),
    "Claude Code · Sonnet · revisa: Claude Code, um modelo acima (com risco)",
  );
  assert.equal(
    choiceSummary({
      ...defaultChoice(),
      reviewer: "codex",
      reviewerModel: "codex",
      reviewWhen: "always",
    }),
    "Claude Code · Sonnet · revisa: Codex (sempre)",
  );
  assert.equal(
    choiceSummary({ ...defaultChoice(), reviewWhen: "never" }),
    "Claude Code · Sonnet · sem revisão",
  );
  assert.equal(modelName("claude", "fable"), "Fable");
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
