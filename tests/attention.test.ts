import assert from "node:assert/strict";
import { test } from "vitest";
import {
  attentionOf,
  byUrgency,
  countPhrase,
  matchesFilter,
  mergeArrivals,
  notificationFor,
} from "../src/lib/attention.ts";
import type { Stage, StageId } from "../src/lib/delivery.ts";
import type { LocalTask } from "../src/lib/desktop.ts";

const task = (id: number, status: LocalTask["status"] = "review"): LocalTask =>
  ({ id, status, title: `t${id}` }) as LocalTask;
const stage = (id: StageId, next: string | null = "Fazer algo"): Stage => ({
  id,
  label: id,
  tone: "info",
  hint: "",
  next,
});
const none = new Set<number>();

test("only tasks that wait for the human need attention, most urgent first", () => {
  const tasks = [1, 2, 3, 4, 5, 6].map((id) => task(id));
  const stages = new Map<number, Stage>([
    [1, stage("ready")],
    [2, stage("checks-failing")],
    [3, stage("checks-running", null)],
    [4, stage("merged", null)],
    [5, stage("local")],
  ]);
  const found = attentionOf(tasks, stages, new Map(), none);
  assert.deepEqual([...found.keys()].sort(), [1, 2, 5]);
  assert.deepEqual(
    byUrgency([...found.values()]).map((item) => item.taskId),
    [2, 1, 5],
  );
  assert.equal(found.get(1)?.reason, "ready · Fazer algo");
});

test("blocked tasks ask for you; working and done tasks never do", () => {
  const tasks = [task(1, "doing"), task(2), task(3, "done"), task(4)];
  const blocked = new Map([[2, { runId: 9, state: "failed", reason: "testes falharam" }]]);
  const stages = new Map<number, Stage>([
    [3, stage("ready")],
    [4, stage("ready")],
  ]);
  const found = attentionOf(tasks, stages, blocked, new Set([4]));
  assert.deepEqual([...found.keys()], [2]);
  assert.match(found.get(2)?.reason ?? "", /Bloqueada: testes falharam/);
});

test("filters split the board between you and the agents", () => {
  const tasks = [task(1), task(2), task(3)];
  const stages = new Map<number, Stage>([
    [1, stage("ready")],
    [2, stage("checks-running", null)],
  ]);
  const working = new Set([3]);
  const attention = attentionOf(tasks, stages, new Map(), working);
  const pick = (filter: "all" | "me" | "agents") =>
    tasks.filter((t) => matchesFilter(filter, t, attention, stages, working)).map((t) => t.id);
  assert.deepEqual(pick("all"), [1, 2, 3]);
  assert.deepEqual(pick("me"), [1]);
  assert.deepEqual(pick("agents"), [2, 3]);
});

test("the notification names the single task or counts several", () => {
  const a = { taskId: 1, title: "A", reason: "Pronto", key: "ready", rank: 3 };
  const b = { taskId: 2, title: "B", reason: "Falhou", key: "x", rank: 1 };
  assert.match(notificationFor([a])?.body ?? "", /#1 A · Pronto/);
  const many = notificationFor([a, b]);
  assert.equal(many?.title, "2 tarefas pedem você");
  assert.match(many?.body ?? "", /#2 B/, "o mais urgente primeiro");
  assert.equal(notificationFor([]), null);
});

test("arrivals while away merge per task and the counter is a full phrase", () => {
  const a = { taskId: 1, title: "A", reason: "r1", key: "k1", rank: 5 };
  const merged = mergeArrivals(
    [a],
    [
      { ...a, reason: "r2", key: "k2" },
      { ...a, taskId: 2 },
    ],
  );
  assert.equal(merged.length, 2);
  assert.equal(merged.find((item) => item.taskId === 1)?.key, "k2");
  assert.equal(countPhrase(0), "Nada pede você agora");
  assert.equal(countPhrase(1), "1 tarefa pede você");
  assert.equal(countPhrase(3), "3 tarefas pedem você");
});
