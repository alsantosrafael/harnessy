import assert from "node:assert/strict";
import { test } from "vitest";
import type { LocalTask, Run } from "../src/lib/desktop.ts";
import { quickStates } from "../src/lib/quick.ts";

const task = (status: LocalTask["status"]): LocalTask => ({
  id: 1,
  workspaceId: 1,
  title: "t",
  description: "",
  criteria: "",
  revision: 1,
  status,
  parentId: null,
  afterId: null,
});
const run = (extra: Partial<Run>): Run =>
  ({
    id: 1,
    taskId: 1,
    state: "completed",
    branch: "harnessy/run-1-x",
    review: null,
    ...extra,
  }) as Run;
const base = { canStart: true, starting: false, dirty: false };
const by = (states: ReturnType<typeof quickStates>) =>
  Object.fromEntries(states.map((state) => [state.id, state.reason]));

test("without runs only implementing and finishing are available", () => {
  const states = by(quickStates({ task: task("todo"), lastRun: null, ...base }));
  assert.equal(states.implement, null);
  assert.equal(states.finish, null);
  assert.match(String(states.review), /Ainda não há run/);
  assert.match(String(states.conversation), /Ainda não há run/);
  assert.match(String(states.branch), /não criou branch/);
});

test("a completed run with a branch can be reviewed, opened and copied", () => {
  const states = by(quickStates({ task: task("review"), lastRun: run({}), ...base }));
  assert.deepEqual([states.review, states.conversation, states.branch], [null, null, null]);
});

test("unsaved edits and active runs block implementing with the reason in text", () => {
  const dirty = by(quickStates({ task: task("todo"), lastRun: null, ...base, dirty: true }));
  assert.match(String(dirty.implement), /Salve a tarefa/);
  const running = by(
    quickStates({
      task: task("doing"),
      lastRun: run({ state: "implementing" }),
      ...base,
      canStart: false,
    }),
  );
  assert.match(String(running.implement), /em andamento/);
  assert.match(String(running.review), /não terminou|Espere/);
});

test("a finished task offers reopening instead of finishing; review says again", () => {
  const states = quickStates({ task: task("done"), lastRun: run({ review: "approve" }), ...base });
  assert.ok(states.some((state) => state.id === "reopen"));
  assert.ok(!states.some((state) => state.id === "finish"));
  assert.equal(states.find((state) => state.id === "review")?.label, "Revisar de novo");
});

test("publishing needs a finished run with a branch; merging needs a safe PR", () => {
  const none = by(quickStates({ task: task("review"), lastRun: null, ...base }));
  assert.match(String(none.publish), /Ainda não há run/);
  assert.match(String(none.merge), /Publique o PR antes/);

  const work = run({});
  const published = {
    runId: 1,
    prNumber: 3,
    prUrl: "u",
    state: "open",
    draft: true,
    headSha: "a",
    checks: "passing",
    mergeable: "mergeable",
    reviewedSha: null,
    publishedAt: 0,
    mergedAt: null,
  } as const;
  const before = quickStates({ task: task("review"), lastRun: work, workRun: work, ...base });
  assert.equal(by(before).publish, null);
  assert.equal(before.find((state) => state.id === "publish")?.label, "Publicar PR");

  const after = quickStates({
    task: task("review"),
    lastRun: work,
    workRun: work,
    delivery: published,
    ...base,
  });
  assert.equal(after.find((state) => state.id === "publish")?.label, "Atualizar PR");
  assert.equal(by(after).merge, null);

  const failing = quickStates({
    task: task("review"),
    lastRun: work,
    workRun: work,
    delivery: { ...published, checks: "failing" },
    ...base,
  });
  assert.match(String(by(failing).merge), /checks reprovados/);
});
