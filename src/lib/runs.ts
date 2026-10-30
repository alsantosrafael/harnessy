import type { LocalTask, Run, RunEvent, StepProposal } from "./desktop";

const labels: Record<string, string> = {
  preparing: "Preparando",
  implementing: "Implementando",
  completed: "Concluída",
  no_changes: "Sem alterações",
  failed: "Falhou",
  cancelled: "Cancelada",
  interrupted: "Interrompida",
};
const terminal = new Set(["completed", "no_changes", "failed", "cancelled", "interrupted"]);
/** Linhas de log mantidas na tela por run. */
const MAX_VISIBLE_EVENTS = 500;

export const stateLabel = (state: string) => labels[state] ?? state;
export const isActive = (run: Run) => !terminal.has(run.state);

/** Troca a run pelo mesmo id ou a coloca no topo, mantendo a lista por id decrescente. */
export function upsertRun(runs: Run[], run: Run): Run[] {
  const rest = runs.filter((item) => item.id !== run.id);
  return [run, ...rest].sort((a, b) => b.id - a.id);
}

/** Junta eventos novos ao log, ignorando repetidos (carga inicial + evento ao vivo). */
export function mergeEvents(current: RunEvent[], incoming: RunEvent[]): RunEvent[] {
  const seen = new Set(current.map((event) => event.seq));
  const merged = [...current, ...incoming.filter((event) => !seen.has(event.seq))];
  merged.sort((a, b) => a.seq - b.seq);
  return merged.slice(-MAX_VISIBLE_EVENTS);
}

/** Tom visual da contraprova: `alert` chama atenção; `muted` é informativo. */
export type ProofTone = "positive" | "alert" | "muted";
const proofs: Record<string, { text: string; tone: ProofTone }> = {
  proved: { text: "Comprovado por testes", tone: "positive" },
  not_proved: { text: "Nenhum teste prova a mudança", tone: "alert" },
  no_tests: { text: "Nenhum teste alterado", tone: "alert" },
  skipped: { text: "Contraprova não executada", tone: "muted" },
  inconclusive: { text: "Contraprova inconclusiva", tone: "muted" },
};

/** Linha de contraprova da run; `null` quando a etapa não rodou ou o veredito é desconhecido. */
export const proofLabel = (proof: string | null) => (proof ? (proofs[proof] ?? null) : null);

/** Botão "Implementar" só com agentes disponíveis e nenhuma run ativa no workspace. */
export const canStartRun = (agentReady: boolean, runs: Run[]) => agentReady && !runs.some(isActive);

/** Ciclo extra com o próximo modelo: run de escrita que falhou e manteve a pasta de trabalho. */
export const canTryNextModel = (run: Run) =>
  run.state === "failed" && run.worktreePath !== null && run.nextModel !== null;

const reviews: Record<string, { text: string; tone: ProofTone }> = {
  approve: { text: "Revisão independente aprovou", tone: "positive" },
  request_changes: { text: "Revisão independente pediu mudanças", tone: "alert" },
  invalid: { text: "Revisor não respondeu no formato esperado", tone: "muted" },
};

/** Linha do revisor, com os achados descartados por estarem fora do diff; `null` sem revisão. */
export function reviewLabel(run: Run): { text: string; tone: ProofTone } | null {
  const label = run.review ? reviews[run.review] : undefined;
  if (!label) return null;
  const discarded = run.reviewDiscarded ?? 0;
  return discarded > 0
    ? { ...label, text: `${label.text} (${discarded} achado(s) fora do diff descartado(s))` }
    : label;
}

/** Passos propostos na run de "Dividir em passos": o último evento `steps`, ou `null`. */
export function stepsFrom(events: readonly RunEvent[]): StepProposal[] | null {
  const last = [...events].reverse().find((event) => event.kind === "steps");
  if (!last) return null;
  try {
    const parsed: unknown = JSON.parse(last.text);
    if (!Array.isArray(parsed)) return null;
    const steps = parsed.filter(
      (item): item is StepProposal =>
        typeof item?.title === "string" && typeof item?.criteria === "string",
    );
    return steps.length > 0 ? steps : null;
  } catch {
    return null;
  }
}

/** Run de onde um passo encadeado continua: a última run concluída, com branch, do passo anterior. */
export function chainBase(task: LocalTask, runs: readonly Run[]): Run | null {
  if (task.afterId === null) return null;
  return (
    runs.find(
      (run) => run.taskId === task.afterId && run.state === "completed" && run.branch !== null,
    ) ?? null
  );
}
