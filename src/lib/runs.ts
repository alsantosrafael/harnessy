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
const MAX_VISIBLE_EVENTS = 2_000;

/** Run de onde um ajuste continua: a concluída mais recente da conversa que deixou branch. */
export const adjustmentBase = (thread: readonly Run[]): Run | null =>
  [...thread].reverse().find((run) => run.state === "completed" && Boolean(run.branch)) ?? null;

/** Quanto texto cabe num ajuste (o núcleo recusa acima disso). */
export const FOLLOWUP_LIMIT = 4_000;

/** Runs da mesma tarefa, da mais antiga à mais nova: a conversa inteira dela. */
export const threadOf = (runs: readonly Run[], taskId: number | null) =>
  taskId === null ? [] : runs.filter((run) => run.taskId === taskId).sort((a, b) => a.id - b.id);

/** Uma conversa por tarefa: a run mais recente e quantas runs ela tem, da mais recente à mais antiga. */
export function conversations(runs: readonly Run[]): { latest: Run; count: number }[] {
  const byTask = new Map<number, { latest: Run; count: number }>();
  for (const run of runs) {
    const current = byTask.get(run.taskId);
    if (!current) byTask.set(run.taskId, { latest: run, count: 1 });
    else
      byTask.set(run.taskId, {
        latest: run.id > current.latest.id ? run : current.latest,
        count: current.count + 1,
      });
  }
  return [...byTask.values()].sort((a, b) => b.latest.id - a.latest.id);
}

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
  const tail = merged.slice(-MAX_VISIBLE_EVENTS);
  // O pedido de quem usa abre a conversa: nunca é cortado junto com o começo do log.
  const asked = merged.filter((event) => event.kind === "user" && !tail.includes(event));
  return [...asked, ...tail];
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

/** "agora", "há 5 min", "há 3 h" ou "há 2 d", a partir de segundos desde a época Unix. */
export function timeAgo(startedAt: number, now = Date.now() / 1000): string {
  const seconds = Math.max(0, Math.round(now - startedAt));
  if (seconds < 60) return "agora";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `há ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `há ${hours} h`;
  return `há ${Math.floor(hours / 24)} d`;
}

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

const NOT_REPORTED = "não informado";
const integer = new Intl.NumberFormat("pt-BR");
const seconds = new Intl.NumberFormat("pt-BR", {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});
const dollars = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 4,
});

/** Contagem do ledger com separador de milhar; `null` é "não informado", nunca zero. */
export const ledgerCount = (value: number | null) =>
  value === null ? NOT_REPORTED : integer.format(value);

/** Duração em milissegundos, mostrada em segundos com uma casa decimal. */
export const ledgerSeconds = (ms: number | null) =>
  ms === null ? NOT_REPORTED : `${seconds.format(ms / 1000)} s`;

/** Menor custo exibível com quatro casas; abaixo disso, um custo real nunca vira "US$ 0,00". */
const SMALLEST_COST = 0.0001;

/** Custo em dólares; `null` é "não informado". Positivo e menor que a precisão mostra "< US$ 0,0001". */
export const ledgerCost = (usd: number | null) => {
  if (usd === null) return NOT_REPORTED;
  if (usd > 0 && usd < SMALLEST_COST / 2) return `< ${dollars.format(SMALLEST_COST)}`;
  return dollars.format(usd);
};

/** Linha do ledger da run: tempos, tokens, custo e ciclos, na ordem em que aparecem na tela. */
export function ledgerItems(run: Run): { label: string; value: string }[] {
  return [
    { label: "Tempo do agente", value: ledgerSeconds(run.agentMs) },
    { label: "Verificações", value: ledgerSeconds(run.checksMs) },
    { label: "Tokens de entrada", value: ledgerCount(run.inputTokens) },
    { label: "Tokens de saída", value: ledgerCount(run.outputTokens) },
    { label: "Custo", value: ledgerCost(run.costUsd) },
    { label: "Ciclos", value: ledgerCount(run.cycles) },
  ];
}

/** Título do aviso de bloqueio, conforme como a run terminou. */
export const blockTitle = (state: string) =>
  state === "no_changes" ? "Sem alterações" : state === "failed" ? "A run falhou" : "Ações negadas";

/** O que "Dispensar aviso" faz e qual é o próximo passo natural. */
export const blockHelp = (state: string) =>
  state === "no_changes"
    ? "Se o trabalho já existe, conclua a tarefa. Dispensar só esconde este aviso; não muda o estado nem inicia nada."
    : "Dispensar só esconde este aviso; não muda o estado nem inicia nada. Para tentar de novo, use Implementar abaixo.";

/** Uma run concluída com alterações na branch pode receber revisão sob demanda. */
export const reviewable = (run: Run) => run.state === "completed" && Boolean(run.branch);
