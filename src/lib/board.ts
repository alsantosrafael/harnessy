import type { LocalTask, Run, TaskChange, TaskStatus } from "./desktop";
import { isActive } from "./runs";

export interface Lane {
  status: TaskStatus;
  name: string;
  /** Texto do estado vazio, dito de forma que diga o que acontece ali. */
  empty: string;
}

/** Raias fixas, na ordem do fluxo. */
export const LANES: readonly Lane[] = [
  { status: "todo", name: "A fazer", empty: "Digite um título acima e tecle Enter." },
  { status: "doing", name: "Em andamento", empty: "Quando o agente começar, o card vem para cá." },
  { status: "review", name: "Revisão", empty: "O que o agente terminar espera por você aqui." },
  { status: "done", name: "Concluído", empty: "Nada concluído ainda." },
];

export const statusName = (status: TaskStatus) =>
  LANES.find((lane) => lane.status === status)?.name ?? status;

/** Agrupa em uma só passada; a ordem de cada raia é a da lista recebida. */
export function groupByStatus(tasks: LocalTask[]): Record<TaskStatus, LocalTask[]> {
  const groups: Record<TaskStatus, LocalTask[]> = { todo: [], doing: [], review: [], done: [] };
  for (const task of tasks) groups[task.status].push(task);
  return groups;
}

/** Move um card na tela antes da confirmação do núcleo. Devolve a mesma lista se nada muda. */
export function applyMove(tasks: LocalTask[], id: number, to: TaskStatus): LocalTask[] {
  if (!tasks.some((task) => task.id === id && task.status !== to)) return tasks;
  return tasks.map((task) => (task.id === id ? { ...task, status: to } : task));
}

/** Aplica o evento do núcleo; ignora tarefas de outro workspace e repetições. */
export function applyChange(tasks: LocalTask[], change: TaskChange): LocalTask[] {
  const task = tasks.find((item) => item.id === change.taskId);
  if (!task || (task.status === change.status && task.revision === change.revision)) return tasks;
  return tasks.map((item) =>
    item.id === change.taskId
      ? { ...item, status: change.status, revision: change.revision }
      : item,
  );
}

/** Raia vizinha (`1` avança, `-1` volta); `null` nas pontas. */
export function neighbor(status: TaskStatus, step: 1 | -1): TaskStatus | null {
  const index = LANES.findIndex((lane) => lane.status === status) + step;
  return LANES[index]?.status ?? null;
}

/** Atalho de teclado no card: `]`/`[` andam uma raia, `1`–`4` vão direto. */
export function shortcutTarget(status: TaskStatus, key: string): TaskStatus | null {
  if (key === "]") return neighbor(status, 1);
  if (key === "[") return neighbor(status, -1);
  const index = Number(key) - 1;
  return Number.isInteger(index) ? (LANES[index]?.status ?? null) : null;
}

export interface Block {
  runId: number;
  reason: string;
}

/**
 * Tarefas bloqueadas: a última run da tarefa terminou com motivo de bloqueio e a
 * tarefa não está concluída. Uma run nova (ativa ou não) supera a anterior.
 */
export function blockedByTask(runs: Run[], tasks: LocalTask[]): Map<number, Block> {
  const done = new Set(tasks.filter((task) => task.status === "done").map((task) => task.id));
  const latest = new Map<number, Run>();
  for (const run of runs) {
    const current = latest.get(run.taskId);
    if (!current || run.id > current.id) latest.set(run.taskId, run);
  }
  const blocked = new Map<number, Block>();
  for (const [taskId, run] of latest) {
    if (!done.has(taskId) && !isActive(run) && run.blocked) {
      blocked.set(taskId, { runId: run.id, reason: run.blocked });
    }
  }
  return blocked;
}
