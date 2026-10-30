import assert from "node:assert/strict";
import { test } from "vitest";
import type { Run, RunEvent } from "../src/lib/desktop.ts";
import {
  canStartRun,
  canTryNextModel,
  chainBase,
  mergeEvents,
  proofLabel,
  reviewLabel,
  timeAgo,
  stateLabel,
  stepsFrom,
  upsertRun,
} from "../src/lib/runs.ts";

test("proof verdicts have a visible line and null stays silent", () => {
  assert.equal(proofLabel(null), null);
  assert.equal(proofLabel("unknown"), null);
  assert.deepEqual(proofLabel("proved"), { text: "Comprovado por testes", tone: "positive" });
  assert.equal(proofLabel("not_proved")?.tone, "alert");
  assert.equal(proofLabel("no_tests")?.text, "Nenhum teste alterado");
  assert.equal(proofLabel("skipped")?.tone, "muted");
});

const run = (id: number, state: string): Run => ({
  id,
  workspaceId: 1,
  taskId: 7,
  title: "t",
  action: "implement",
  role: "implementador",
  runtime: "claude",
  state,
  baseSha: null,
  branch: null,
  worktreePath: null,
  startedAt: 0,
  finishedAt: null,
  summary: "",
  blocked: "",
  model: null,
  cycles: null,
  turns: null,
  inputTokens: null,
  outputTokens: null,
  cacheReadTokens: null,
  cacheWriteTokens: null,
  costUsd: null,
  agentMs: null,
  checksMs: null,
  proof: null,
  effort: null,
  lastModel: null,
  nextModel: null,
  review: null,
  reviewDiscarded: null,
  baseRunId: null,
  reviewer: null,
  reviewerModel: null,
  reviewWhen: null,
});
const event = (seq: number): RunEvent => ({ runId: 1, seq, kind: "log", text: String(seq) });

test("a new state replaces the run and keeps newest first", () => {
  const runs = upsertRun([run(1, "preparing")], run(2, "preparing"));
  assert.deepEqual(
    upsertRun(runs, run(1, "completed")).map((item) => [item.id, item.state]),
    [
      [2, "preparing"],
      [1, "completed"],
    ],
  );
});

test("events from the initial load and the live stream do not duplicate", () => {
  const merged = mergeEvents([event(1), event(2)], [event(2), event(3)]);
  assert.deepEqual(
    merged.map((item) => item.seq),
    [1, 2, 3],
  );
});

test("a run starts only with agents available and no active run", () => {
  assert.equal(canStartRun(true, [run(1, "completed")]), true);
  assert.equal(canStartRun(true, [run(1, "implementing")]), false);
  assert.equal(canStartRun(false, []), false);
  assert.equal(stateLabel("no_changes"), "Sem alterações");
});

test("next model is offered only for a failed run that kept its folder and is below the top", () => {
  const failed = {
    ...run(1, "failed"),
    worktreePath: "/w",
    lastModel: "haiku",
    nextModel: "sonnet",
  };
  assert.equal(canTryNextModel(failed), true);
  assert.equal(canTryNextModel({ ...failed, nextModel: null }), false);
  assert.equal(canTryNextModel({ ...failed, worktreePath: null }), false);
  assert.equal(canTryNextModel({ ...failed, state: "completed" }), false);
});

test("review line shows the verdict and discarded findings; no review stays silent", () => {
  assert.equal(reviewLabel(run(1, "completed")), null);
  const reviewed = { ...run(1, "completed"), review: "approve", reviewDiscarded: 2 };
  assert.equal(reviewLabel(reviewed)?.tone, "positive");
  assert.match(reviewLabel(reviewed)?.text ?? "", /2 achado\(s\) fora do diff/);
  assert.equal(reviewLabel({ ...reviewed, review: "request_changes" })?.tone, "alert");
});

test("steps come from the last valid steps event", () => {
  const steps = (seq: number, text: string): RunEvent => ({ runId: 1, seq, kind: "steps", text });
  assert.equal(stepsFrom([event(1)]), null);
  assert.equal(stepsFrom([steps(1, "não é json")]), null);
  assert.deepEqual(stepsFrom([steps(1, '[{"title":"A","criteria":"- a"},{"x":1}]')]), [
    { title: "A", criteria: "- a" },
  ]);
});

test("a chained step continues from the latest completed run of the previous task", () => {
  const step = {
    id: 9,
    workspaceId: 1,
    title: "Passo 2",
    description: "",
    criteria: "",
    revision: 1,
    status: "todo" as const,
    parentId: 7,
    afterId: 8,
  };
  const done = { ...run(5, "completed"), taskId: 8, branch: "harnessy/run-5-passo-1" };
  const failed = { ...run(6, "failed"), taskId: 8, branch: "harnessy/run-6-passo-1" };
  assert.equal(chainBase(step, [failed, done])?.id, 5);
  assert.equal(chainBase({ ...step, afterId: null }, [done]), null);
  assert.equal(chainBase(step, [failed]), null);
});

test("the run list shows elapsed time in short Portuguese units", () => {
  const now = 1_000_000;
  assert.equal(timeAgo(now - 20, now), "agora");
  assert.equal(timeAgo(now - 5 * 60, now), "há 5 min");
  assert.equal(timeAgo(now - 3 * 3600, now), "há 3 h");
  assert.equal(timeAgo(now - 2 * 86400, now), "há 2 d");
  assert.equal(timeAgo(now + 10, now), "agora");
});
