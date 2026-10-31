import type { Block } from "./board";
import type { Stage, StageId } from "./delivery";
import type { LocalTask } from "./desktop";

/** O que pede o humano numa tarefa, e quão cedo. */
export interface Attention {
  taskId: number;
  title: string;
  /** Frase curta: o estado e a ação esperada. */
  reason: string;
  /** Muda quando o motivo muda; serve para saber o que é novidade. */
  key: string;
  /** Menor = mais urgente. */
  rank: number;
}

export type Filter = "all" | "me" | "agents";

const RANK: Partial<Record<StageId, number>> = {
  "checks-failing": 1,
  conflict: 1,
  "changes-requested": 2,
  ready: 3,
  draft: 4,
  open: 4,
  local: 5,
  closed: 6,
};
const BLOCKED_RANK = 1;

/**
 * Tarefas que esperam o humano. Concluída nunca pede; PR mergeado e checks rodando
 * são trabalho de outros (GitHub, agentes), então ficam de fora.
 */
export function attentionOf(
  tasks: LocalTask[],
  stages: ReadonlyMap<number, Stage>,
  blocked: ReadonlyMap<number, Block>,
  working: ReadonlySet<number>,
): Map<number, Attention> {
  const found = new Map<number, Attention>();
  for (const task of tasks) {
    if (task.status === "done" || working.has(task.id)) continue;
    const block = blocked.get(task.id);
    if (block) {
      found.set(task.id, {
        taskId: task.id,
        title: task.title,
        reason: `Bloqueada: ${block.reason}`,
        key: `blocked:${block.runId}`,
        rank: BLOCKED_RANK,
      });
      continue;
    }
    const stage = stages.get(task.id);
    const rank = stage ? RANK[stage.id] : undefined;
    if (!stage || rank === undefined) continue;
    found.set(task.id, {
      taskId: task.id,
      title: task.title,
      reason: stage.next ? `${stage.label} · ${stage.next}` : stage.label,
      key: stage.id,
      rank,
    });
  }
  return found;
}

/** A tarefa está com agentes ou com o GitHub: nada a fazer além de esperar. */
export const withAgents = (
  task: LocalTask,
  stages: ReadonlyMap<number, Stage>,
  working: ReadonlySet<number>,
) => working.has(task.id) || stages.get(task.id)?.id === "checks-running";

export function matchesFilter(
  filter: Filter,
  task: LocalTask,
  attention: ReadonlyMap<number, Attention>,
  stages: ReadonlyMap<number, Stage>,
  working: ReadonlySet<number>,
): boolean {
  if (filter === "all") return true;
  if (filter === "me") return attention.has(task.id);
  return withAgents(task, stages, working);
}

/** Do mais urgente ao menos; em empate, a tarefa mais antiga primeiro. */
export const byUrgency = (items: Attention[]) =>
  [...items].sort((a, b) => a.rank - b.rank || a.taskId - b.taskId);

/** Frase do contador: "3 tarefas pedem você". */
export function countPhrase(count: number): string {
  if (count === 0) return "Nada pede você agora";
  return count === 1 ? "1 tarefa pede você" : `${count} tarefas pedem você`;
}

/** Texto da notificação do sistema para o que acabou de chegar. */
export function notificationFor(arrived: Attention[]): { title: string; body: string } | null {
  const [first] = byUrgency(arrived);
  if (!first) return null;
  if (arrived.length === 1) {
    return { title: "Precisa de você", body: `#${first.taskId} ${first.title} · ${first.reason}` };
  }
  return {
    title: `${arrived.length} tarefas pedem você`,
    body: `Primeiro: #${first.taskId} ${first.title} · ${first.reason}`,
  };
}

/** Junta o que chegou enquanto a janela estava sem foco; a novidade mais recente de cada tarefa vence. */
export function mergeArrivals(current: Attention[], arrived: Attention[]): Attention[] {
  const byTask = new Map(current.map((item) => [item.taskId, item]));
  for (const item of arrived) byTask.set(item.taskId, item);
  return [...byTask.values()];
}
