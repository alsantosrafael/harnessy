import assert from "node:assert/strict";
import { test } from "vitest";
import {
  applyChange,
  blockedByTask,
  applyMove,
  groupByStatus,
  LANES,
  neighbor,
  shortcutTarget,
} from "../src/lib/board.ts";
import type { LocalTask, Run, TaskChange, TaskStatus } from "../src/lib/desktop.ts";

const task = (id: number, status: TaskStatus): LocalTask => ({
  id,
  workspaceId: 1,
  title: `t${id}`,
  description: "",
  criteria: "",
  revision: 1,
  status,
  parentId: null,
  afterId: null,
});
const tasks = [task(3, "todo"), task(2, "review"), task(1, "todo")];

test("lanes are fixed and follow the flow", () => {
  assert.deepEqual(
    LANES.map((lane) => lane.status),
    ["todo", "doing", "review", "done"],
  );
});

test("grouping keeps the received order inside each lane", () => {
  const groups = groupByStatus(tasks);
  assert.deepEqual(
    groups.todo.map((item) => item.id),
    [3, 1],
  );
  assert.deepEqual(
    groups.review.map((item) => item.id),
    [2],
  );
  assert.equal(groups.doing.length + groups.done.length, 0);
});

test("moving changes only that card and is immutable", () => {
  const moved = applyMove(tasks, 3, "doing");
  assert.equal(moved.find((item) => item.id === 3)?.status, "doing");
  assert.equal(moved.find((item) => item.id === 1)?.status, "todo");
  assert.equal(tasks[0].status, "todo");
});

test("a move to the same lane or an unknown card returns the same list", () => {
  assert.equal(applyMove(tasks, 3, "todo"), tasks);
  assert.equal(applyMove(tasks, 99, "done"), tasks);
});

test("a core event patches status and revision, and repeats are ignored", () => {
  const change: TaskChange = {
    taskId: 2,
    status: "done",
    revision: 1,
    event: { seq: 3, at: 0, fromStatus: "review", toStatus: "done", actor: "system", note: "" },
  };
  const patched = applyChange(tasks, change);
  assert.equal(patched.find((item) => item.id === 2)?.status, "done");
  assert.equal(applyChange(patched, change), patched);
  assert.equal(applyChange(tasks, { ...change, taskId: 99 }), tasks);
});

test("neighbors stop at the ends and shortcuts map to lanes", () => {
  assert.equal(neighbor("todo", 1), "doing");
  assert.equal(neighbor("todo", -1), null);
  assert.equal(neighbor("done", 1), null);
  assert.equal(shortcutTarget("doing", "]"), "review");
  assert.equal(shortcutTarget("doing", "["), "todo");
  assert.equal(shortcutTarget("todo", "4"), "done");
  assert.equal(shortcutTarget("todo", "5"), null);
  assert.equal(shortcutTarget("todo", "x"), null);
});

const run = (id: number, taskId: number, state: string, blocked = ""): Run => ({
  id,
  workspaceId: 1,
  taskId,
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
  blocked,
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

test("only the latest finished run of an open task can block it", () => {
  const open = [task(1, "todo"), task(2, "review"), task(3, "done"), task(4, "todo")];
  const runs = [
    run(1, 1, "failed", "Run falhou."),
    run(2, 2, "completed", "Ações negadas: Bash: git push"),
    run(3, 3, "failed", "Run falhou."),
    run(4, 4, "failed", "Run falhou: antiga"),
    run(5, 4, "implementing"),
  ];
  const blocked = blockedByTask(runs, open);
  assert.deepEqual([...blocked.keys()], [1, 2]);
  assert.deepEqual(blocked.get(1), { runId: 1, state: "failed", reason: "Run falhou." });
  assert.equal(blocked.has(3), false, "concluída não mostra bloqueio");
  assert.equal(blocked.has(4), false, "run nova supera a anterior");
});

test("a resolved run no longer blocks", () => {
  assert.equal(blockedByTask([run(1, 1, "failed", "")], [task(1, "todo")]).size, 0);
});
