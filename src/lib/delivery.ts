import type { Delivery, LocalTask, Run } from "./desktop";
import { isActive } from "./runs";

export type StageId =
  | "local"
  | "draft"
  | "open"
  | "checks-running"
  | "checks-failing"
  | "conflict"
  | "changes-requested"
  | "ready"
  | "merged"
  | "closed";

export type StageTone = "neutral" | "info" | "warn" | "critical" | "success";

/** Onde a tarefa está na linha de entrega, dito para quem observa o quadro. */
export interface Stage {
  id: StageId;
  label: string;
  tone: StageTone;
  /** Frase completa para tooltip e leitor de tela: o que está acontecendo e o que fazer. */
  hint: string;
  /** Próxima ação do humano; `null` quando não há nada a fazer agora. */
  next: string | null;
}

const STAGES: Record<StageId, Omit<Stage, "id">> = {
  local: {
    label: "Só local",
    tone: "neutral",
    hint: "A branch existe só neste computador. Publique o PR para continuar.",
    next: "Publicar PR",
  },
  draft: {
    label: "PR em draft",
    tone: "info",
    hint: "PR aberto como draft, ainda sem revisão aprovada.",
    next: "Revisar",
  },
  open: {
    label: "PR aberto",
    tone: "info",
    hint: "PR aberto, ainda sem revisão aprovada.",
    next: "Revisar",
  },
  "checks-running": {
    label: "Checks rodando",
    tone: "info",
    hint: "Os checks do PR ainda estão rodando.",
    next: null,
  },
  "checks-failing": {
    label: "Checks falhando",
    tone: "critical",
    hint: "Há checks reprovados no PR. Peça um ajuste na mesma run.",
    next: "Corrigir checks",
  },
  conflict: {
    label: "Conflito",
    tone: "critical",
    hint: "O PR tem conflitos com a base. Peça um ajuste para resolvê-los.",
    next: "Resolver conflito",
  },
  "changes-requested": {
    label: "Mudanças pedidas",
    tone: "warn",
    hint: "A revisão pediu mudanças e nada foi corrigido desde então.",
    next: "Corrigir achados",
  },
  ready: {
    label: "Pronto para merge",
    tone: "success",
    hint: "Checks sem falha, sem conflito e revisão aprovada neste commit.",
    next: "Fazer merge",
  },
  merged: {
    label: "Mergeado",
    tone: "success",
    hint: "O PR foi mesclado.",
    next: null,
  },
  closed: {
    label: "PR fechado",
    tone: "warn",
    hint: "O PR foi fechado sem merge.",
    next: "Publicar de novo",
  },
};

const stage = (id: StageId): Stage => ({ id, ...STAGES[id] });

/** A aprovação do revisor vale para o commit que está no PR agora. */
export const reviewIsCurrent = (delivery: Delivery) =>
  delivery.reviewedSha !== null && delivery.reviewedSha === delivery.headSha;

/**
 * Estágio de uma tarefa. `run` é a run que carrega o trabalho (a mais recente com
 * branch) e `delivery` o PR dela, se existir. `null` quando não há o que mostrar.
 */
export function stageOf(run: Run | null, delivery: Delivery | null): Stage | null {
  if (!run || isActive(run)) return null;
  if (delivery) {
    if (delivery.state === "merged") return stage("merged");
    if (delivery.state === "closed") return stage("closed");
    if (delivery.mergeable === "conflicting") return stage("conflict");
    if (delivery.checks === "failing") return stage("checks-failing");
    if (run.review === "request_changes" && !reviewIsCurrent(delivery)) {
      return stage("changes-requested");
    }
    if (delivery.checks === "pending") return stage("checks-running");
    if (run.review === "approve" && reviewIsCurrent(delivery)) return stage("ready");
    return stage(delivery.draft ? "draft" : "open");
  }
  return run.state === "completed" && run.branch ? stage("local") : null;
}

/** Por que o merge não deve acontecer agora (espelha o núcleo), ou `null` se está liberado. */
export function mergeBlocker(run: Run | null, delivery: Delivery | null): string | null {
  if (!delivery) return "Publique o PR antes.";
  if (delivery.state !== "open") return "O PR não está aberto.";
  if (delivery.checks === "failing") return "Há checks reprovados no PR.";
  if (delivery.checks === "pending") return "Os checks do PR ainda estão rodando.";
  if (delivery.mergeable === "conflicting") return "O PR tem conflitos com a base.";
  if (run?.review === "request_changes" && !reviewIsCurrent(delivery)) {
    return "A revisão pediu mudanças e nada foi corrigido desde então.";
  }
  return null;
}

/** Run que carrega o trabalho da tarefa: a mais recente que deixou branch. */
export function workRun(runs: Run[], taskId: number): Run | null {
  let best: Run | null = null;
  for (const run of runs) {
    if (run.taskId !== taskId || !run.branch) continue;
    if (!best || run.id > best.id) best = run;
  }
  return best;
}

/** PR de cada tarefa, indexado pela run que o publicou. */
export function deliveryOf(deliveries: Delivery[], run: Run | null): Delivery | null {
  return run ? (deliveries.find((item) => item.runId === run.id) ?? null) : null;
}

/** Estágio de cada tarefa que tem algo a mostrar. Tarefas em A fazer/Andamento sem PR ficam de fora. */
export function stagesByTask(
  tasks: LocalTask[],
  runs: Run[],
  deliveries: Delivery[],
): Map<number, Stage> {
  const stages = new Map<number, Stage>();
  for (const task of tasks) {
    const run = workRun(runs, task.id);
    const delivery = deliveryOf(deliveries, run);
    // Concluído só mostra o selo quando o PR foi mesclado; "Só local" ali seria ruído.
    if (task.status === "done" && delivery?.state !== "merged") continue;
    if (task.status === "todo" || task.status === "doing") continue;
    const found = stageOf(run, delivery);
    if (found) stages.set(task.id, found);
  }
  return stages;
}

const ORDER: StageId[] = [
  "ready",
  "checks-failing",
  "conflict",
  "changes-requested",
  "checks-running",
  "draft",
  "open",
  "local",
  "closed",
  "merged",
];

/** Contagem por estágio para o cabeçalho de uma raia: o que pede ação vem primeiro. */
export function summarize(stages: Stage[]): { id: StageId; label: string; count: number }[] {
  const counts = new Map<StageId, number>();
  for (const item of stages) counts.set(item.id, (counts.get(item.id) ?? 0) + 1);
  return ORDER.filter((id) => counts.has(id)).map((id) => ({
    id,
    label: STAGES[id].label,
    count: counts.get(id) ?? 0,
  }));
}

/** Há PR aberto que vale reler no GitHub (checks e conflitos mudam por fora). */
export const needsRefresh = (delivery: Delivery) => delivery.state === "open";
