import assert from "node:assert/strict";
import { test } from "vitest";
import type { LocalTask, Run } from "../src/lib/desktop.ts";
import { nextAction, quickStates } from "../src/lib/quick.ts";

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

test("a finished task locks the actions that change the work", () => {
  const states = by(
    quickStates({ task: task("done"), lastRun: run({ review: "approve" }), ...base }),
  );
  for (const id of ["implement", "review", "preview", "publish", "merge"] as const) {
    assert.match(String(states[id]), /Tarefa concluída/);
  }
  assert.equal(states.reopen, null);
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

test("running locally needs a finished run with changes and flips to stopping when live", () => {
  const none = by(quickStates({ task: task("review"), lastRun: null, ...base }));
  assert.match(String(none.preview), /Ainda não há run/);
  const work = run({});
  const off = quickStates({ task: task("review"), lastRun: work, workRun: work, ...base });
  assert.equal(off.find((state) => state.id === "preview")?.label, "Rodar localmente");
  assert.equal(by(off).preview, null);
  const on = quickStates({
    task: task("review"),
    lastRun: work,
    workRun: work,
    preview: {
      runId: 1,
      state: "running",
      url: "http://localhost:5173",
      exitCode: null,
      startedAt: 0,
    },
    ...base,
  });
  assert.equal(on.find((state) => state.id === "preview")?.label, "Parar ambiente");
});

const pr = (extra: Record<string, unknown> = {}) =>
  ({
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
  }) as never;
const next = (extra: Record<string, unknown> = {}, status: LocalTask["status"] = "review") => {
  const work = (extra.work as Run | undefined) ?? run({});
  return nextAction({
    task: task(status),
    lastRun: work,
    workRun: work,
    ...base,
    ...extra,
  } as never)?.id;
};

test("the next step follows the delivery line, one action at a time", () => {
  assert.equal(nextAction({ task: task("todo"), lastRun: null, ...base })?.id, "implement");
  assert.equal(next({}), "preview", "primeiro ver a mudança funcionando");
  assert.equal(next({ previewSeen: true }), "publish", "depois de testar, publicar");
  assert.equal(next({ delivery: pr() }), "review", "PR sem revisão aprovada");
  assert.equal(
    next({ delivery: pr({ checks: "pending" }) }),
    "publish",
    "atualizar enquanto os checks rodam",
  );
  assert.equal(next({ delivery: pr({ checks: "failing" }) }), "conversation", "pedir ajuste");
  const approved = run({ review: "approve" });
  assert.equal(
    next({ work: approved, delivery: pr({ reviewedSha: "a", checks: "passing" }) }),
    "merge",
  );
});

test("there is no next step while the agent works, after merge, or when done", () => {
  assert.equal(next({ work: run({ state: "implementing" }) }), undefined);
  assert.equal(
    nextAction({
      task: task("review"),
      lastRun: run({}),
      starting: true,
      canStart: true,
      dirty: false,
    }),
    null,
  );
  assert.equal(next({ delivery: pr({ state: "merged" }) }), undefined);
  assert.equal(next({}, "done"), undefined);
});

test("a run that delivered nothing asks to try again", () => {
  const failed = run({ state: "failed", branch: null });
  const action = nextAction({ task: task("review"), lastRun: failed, ...base });
  assert.equal(action?.id, "implement");
  assert.equal(action?.label, "Tentar de novo");
});
