import assert from "node:assert/strict";
import { test } from "vitest";
import {
  mergeBlocker,
  needsRefresh,
  stageOf,
  stagesByTask,
  summarize,
  workRun,
} from "../src/lib/delivery.ts";
import type { Delivery, LocalTask, Run } from "../src/lib/desktop.ts";

const run = (extra: Partial<Run> = {}): Run =>
  ({
    id: 1,
    taskId: 1,
    state: "completed",
    branch: "harnessy/run-1-x",
    review: null,
    ...extra,
  }) as Run;
const pr = (extra: Partial<Delivery> = {}): Delivery => ({
  runId: 1,
  prNumber: 3,
  prUrl: "u",
  state: "open",
  draft: true,
  headSha: "a",
  checks: "none",
  mergeable: "unknown",
  reviewedSha: null,
  publishedAt: 0,
  mergedAt: null,
  ...extra,
});
const task = (id: number, status: LocalTask["status"]): LocalTask =>
  ({ id, status, title: "t" }) as LocalTask;

test("a finished run without a PR is local only and asks to publish", () => {
  const stage = stageOf(run(), null);
  assert.equal(stage?.id, "local");
  assert.equal(stage?.next, "Publicar PR");
  assert.equal(stageOf(run({ branch: null }), null), null);
  assert.equal(stageOf(run({ state: "implementing" }), null), null);
});

test("PR stages follow conflicts, then checks, then review", () => {
  assert.equal(stageOf(run(), pr())?.id, "draft");
  assert.equal(stageOf(run(), pr({ draft: false }))?.id, "open");
  assert.equal(stageOf(run(), pr({ checks: "pending" }))?.id, "checks-running");
  assert.equal(stageOf(run(), pr({ checks: "failing", mergeable: "conflicting" }))?.id, "conflict");
  assert.equal(stageOf(run(), pr({ checks: "failing" }))?.id, "checks-failing");
  assert.equal(stageOf(run({ review: "request_changes" }), pr())?.id, "changes-requested");
});

test("ready needs an approval for the commit that is in the PR now", () => {
  const approved = run({ review: "approve" });
  assert.equal(stageOf(approved, pr({ reviewedSha: "a", checks: "passing" }))?.id, "ready");
  assert.equal(stageOf(approved, pr({ reviewedSha: "old", checks: "passing" }))?.id, "draft");
  assert.equal(
    stageOf(approved, pr({ reviewedSha: "a", checks: "pending" }))?.id,
    "checks-running",
  );
});

test("merged and closed PRs are terminal stages", () => {
  assert.equal(stageOf(run(), pr({ state: "merged" }))?.id, "merged");
  assert.equal(stageOf(run(), pr({ state: "closed" }))?.id, "closed");
  assert.equal(needsRefresh(pr()), true);
  assert.equal(needsRefresh(pr({ state: "merged" })), false);
});

test("merge blocker mirrors the core rules", () => {
  assert.match(String(mergeBlocker(run(), null)), /Publique/);
  assert.equal(mergeBlocker(run(), pr({ checks: "passing", mergeable: "mergeable" })), null);
  assert.match(String(mergeBlocker(run(), pr({ checks: "failing" }))), /checks reprovados/);
  assert.match(String(mergeBlocker(run(), pr({ checks: "pending" }))), /rodando/);
  assert.match(String(mergeBlocker(run(), pr({ mergeable: "conflicting" }))), /conflitos/);
  assert.match(String(mergeBlocker(run({ review: "request_changes" }), pr())), /pediu mudanças/);
});

test("the work run is the newest one with a branch", () => {
  const runs = [
    run({ id: 1 }),
    run({ id: 3 }),
    run({ id: 4, branch: null }),
    run({ id: 2, taskId: 9 }),
  ];
  assert.equal(workRun(runs, 1)?.id, 3);
  assert.equal(workRun(runs, 7), null);
});

test("only review and merged-done tasks get a stage, and lanes summarize by urgency", () => {
  const runs = [run({ id: 1, taskId: 1 }), run({ id: 2, taskId: 2 }), run({ id: 3, taskId: 3 })];
  const stages = stagesByTask([task(1, "review"), task(2, "done"), task(3, "doing")], runs, [
    pr({ runId: 2, state: "merged" }),
  ]);
  assert.equal(stages.get(1)?.id, "local");
  assert.equal(stages.get(2)?.id, "merged");
  assert.equal(stages.has(3), false);
  const summary = summarize([...stages.values(), ...stages.values()]);
  assert.deepEqual(
    summary.map((item) => [item.id, item.count]),
    [
      ["local", 2],
      ["merged", 2],
    ],
  );
});
