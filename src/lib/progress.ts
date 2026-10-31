import type { Run, RunEvent } from "./desktop";

const VERBS: Record<string, string> = {
  Read: "Lendo",
  Grep: "Procurando em",
  Glob: "Procurando em",
  Edit: "Editando",
  MultiEdit: "Editando",
  Write: "Escrevendo",
  NotebookEdit: "Editando",
  Bash: "Rodando",
  TodoWrite: "Planejando",
};

const MAX_DETAIL = 60;

/** Os dois últimos trechos de um caminho; comandos longos são cortados. */
function shorten(detail: string): string {
  const parts = detail.split("/");
  const text = parts.length > 2 && detail.startsWith("/") ? parts.slice(-2).join("/") : detail;
  return text.length > MAX_DETAIL ? `${text.slice(0, MAX_DETAIL - 1)}…` : text;
}

/** "Edit: /a/b/c.md" vira "Editando b/c.md". Sem verbo conhecido, devolve o texto como veio. */
export function describeTool(text: string): string {
  const at = text.indexOf(": ");
  const name = at === -1 ? text : text.slice(0, at);
  const detail = at === -1 ? "" : text.slice(at + 2);
  const verb = VERBS[name];
  if (!verb) return text;
  return detail ? `${verb} ${shorten(detail)}` : verb;
}

/** O que o agente está fazendo agora: a última ação ou etapa registrada. */
export function activityOf(events: readonly RunEvent[] | undefined): string | null {
  for (let i = (events?.length ?? 0) - 1; i >= 0; i--) {
    const event = events?.[i];
    if (event?.kind === "tool") return describeTool(event.text);
    if (event?.kind === "status") return event.text;
  }
  return null;
}

/** Tempo decorrido como `m:ss`. */
export function elapsedLabel(startedAtSeconds: number, nowMs: number): string {
  const total = Math.max(0, Math.floor(nowMs / 1000 - startedAtSeconds));
  const minutes = Math.floor(total / 60);
  return `${minutes}:${String(total % 60).padStart(2, "0")}`;
}

/** Etapa da run em palavras. */
export function phaseOf(run: Pick<Run, "state">): string {
  return run.state === "preparing" ? "Preparando a cópia do projeto" : "Implementando";
}

export interface Progress {
  phase: string;
  activity: string | null;
  elapsed: string;
}

export function progressOf(
  run: Run,
  events: readonly RunEvent[] | undefined,
  nowMs: number,
): Progress {
  return {
    phase: phaseOf(run),
    activity: activityOf(events),
    elapsed: elapsedLabel(run.startedAt, nowMs),
  };
}
