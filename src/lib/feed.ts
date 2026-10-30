import type { RunEvent } from "./desktop";

/** Achado do revisor, já filtrado pelo núcleo para linhas do diff. */
export interface Finding {
  arquivo: string;
  linha: number;
  severidade: string;
  texto: string;
}

/** Evento `review`: quem revisou, o veredito e os achados que sobraram. */
export interface ReviewPayload {
  reviewer: string;
  verdict: string;
  blocking: Finding[];
  suggestions: Finding[];
  discarded: number;
}

export type FeedItem =
  | { kind: "message"; seq: number; text: string }
  | { kind: "tools"; seq: number; lines: string[] }
  | { kind: "status"; seq: number; text: string }
  | { kind: "denied"; seq: number; text: string }
  | { kind: "result"; seq: number; text: string }
  | { kind: "review"; seq: number; review: ReviewPayload }
  | { kind: "logs"; seq: number; lines: string[] };

const isFindings = (value: unknown): value is Finding[] =>
  Array.isArray(value) &&
  value.every((item) => typeof item?.arquivo === "string" && typeof item?.texto === "string");

/** Lê o JSON do evento `review`; `null` se vier fora do formato. */
export function parseReview(text: string): ReviewPayload | null {
  try {
    const raw = JSON.parse(text);
    if (typeof raw?.verdict !== "string") return null;
    return {
      reviewer: typeof raw.reviewer === "string" ? raw.reviewer : "",
      verdict: raw.verdict,
      blocking: isFindings(raw.blocking) ? raw.blocking : [],
      suggestions: isFindings(raw.suggestions) ? raw.suggestions : [],
      discarded: typeof raw.discarded === "number" ? raw.discarded : 0,
    };
  } catch {
    return null;
  }
}

/** Eventos que não entram na conversa: a proposta de passos tem tela própria. */
const HIDDEN = new Set(["steps"]);
/** Ações do agente e logs técnicos: agrupados quando seguidos, para não encher a tela. */
const GROUPED = {
  tool: "tools",
  log: "logs",
  stderr: "logs",
  failures: "logs",
  proof: "logs",
} as const;

/** Transforma o log da run numa conversa: mensagens do agente, ações agrupadas e revisão. */
export function buildFeed(events: readonly RunEvent[]): FeedItem[] {
  const items: FeedItem[] = [];
  for (const event of events) {
    if (HIDDEN.has(event.kind)) continue;
    const group = GROUPED[event.kind as keyof typeof GROUPED];
    if (group) {
      const last = items[items.length - 1];
      if (last?.kind === group) last.lines.push(event.text);
      else items.push({ kind: group, seq: event.seq, lines: [event.text] });
      continue;
    }
    switch (event.kind) {
      case "assistant":
        items.push({ kind: "message", seq: event.seq, text: event.text });
        break;
      case "review": {
        const review = parseReview(event.text);
        if (review) items.push({ kind: "review", seq: event.seq, review });
        break;
      }
      case "denied":
        items.push({ kind: "denied", seq: event.seq, text: event.text });
        break;
      case "result":
        items.push({ kind: "result", seq: event.seq, text: event.text });
        break;
      default:
        items.push({ kind: "status", seq: event.seq, text: event.text });
    }
  }
  return items;
}
