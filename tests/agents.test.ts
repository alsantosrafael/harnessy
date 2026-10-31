import assert from "node:assert/strict";
import { test } from "vitest";
import {
  agentOf,
  agents,
  choiceSummary,
  defaultChoice,
  effortsOf,
  modelName,
  normalizeChoice,
  validEffort,
} from "../src/lib/agents.ts";
import { splitDraft } from "../src/lib/chatTask.ts";

test("the default is Claude Code with Sonnet, reviewed by Claude Code on every run", () => {
  assert.deepEqual(defaultChoice(), {
    runtime: "claude",
    model: "sonnet",
    effort: "",
    reviewer: "claude",
    reviewerModel: "",
    reviewerEffort: "",
    reviewWhen: "always",
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
    effort: "",
    reviewer: "codex",
    reviewerModel: "codex",
    reviewerEffort: "",
    reviewWhen: "always",
  });
  assert.equal(normalizeChoice({ runtime: "codex", model: "--sandbox x" }).model, "codex");
  assert.equal(normalizeChoice({ runtime: "codex", model: "" }).model, "codex");
});

test("the Claude reviewer may stay automatic; unknown policies fall back to always", () => {
  assert.equal(normalizeChoice({ reviewer: "claude", reviewerModel: "" }).reviewerModel, "");
  assert.equal(
    normalizeChoice({ reviewer: "claude", reviewerModel: "opus" }).reviewerModel,
    "opus",
  );
  assert.equal(normalizeChoice({ reviewWhen: "talvez" }).reviewWhen, "always");
  assert.equal(normalizeChoice({ reviewWhen: "auto" }).reviewWhen, "auto");
});

test("the summary says who implements, who reviews and when", () => {
  assert.equal(
    choiceSummary(defaultChoice()),
    "Claude Code · Sonnet · revisa: Claude Code, um modelo acima (sempre)",
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

test("effort is valid per agent and the codex list is shorter than the claude one", () => {
  assert.deepEqual(
    effortsOf("claude").map((e) => e.id),
    ["low", "medium", "high", "xhigh", "max"],
  );
  assert.deepEqual(
    effortsOf("codex").map((e) => e.id),
    ["low", "medium", "high"],
  );
  assert.equal(validEffort("claude", "max"), "max");
  assert.equal(validEffort("codex", "max"), "", "xhigh e max não existem no Codex");
  assert.equal(validEffort("claude", "altissimo"), "");
  assert.equal(validEffort("claude", 7), "");
});

test("a saved choice keeps the effort of each side and drops what the agent does not know", () => {
  const choice = normalizeChoice({
    runtime: "claude",
    model: "sonnet",
    effort: "high",
    reviewer: "codex",
    reviewerModel: "codex",
    reviewerEffort: "max",
  });
  assert.equal(choice.effort, "high");
  assert.equal(choice.reviewerEffort, "", "max não vale para o Codex");
  assert.equal(normalizeChoice({}).effort, "");
  assert.equal(normalizeChoice({}).reviewerEffort, "");
});

test("the summary names the effort only when it was chosen", () => {
  const base = normalizeChoice({});
  assert.doesNotMatch(choiceSummary(base), /esforço/);
  const careful = { ...base, effort: "high", reviewerEffort: "low" };
  assert.match(choiceSummary(careful), /Sonnet · esforço alto/);
  assert.match(choiceSummary(careful), /um modelo acima · esforço baixo/);
});
