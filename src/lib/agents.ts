import type { Runtime } from "./roles";

export interface ModelOption {
  id: string;
  label: string;
}

export interface AgentOption {
  runtime: Runtime;
  label: string;
  /** Modelos conhecidos; com `customModel`, a interface aceita também um id livre. */
  models: readonly ModelOption[];
  defaultModel: string;
  /** O agente aceita um id de modelo digitado (os ids dele variam por conta). */
  customModel?: boolean;
}

/**
 * Agentes e modelos de quem implementa e de quem revisa. Os ids do Claude Code são
 * aliases do `claude --model` e precisam estar também em `MODELS`, em
 * `src-tauri/src/run/guard.rs`, que é quem valida a escolha. O Codex usa o modelo
 * da configuração dele (`codex`) ou um id digitado, validado em `guard::codex_model`.
 */
export const agents: readonly AgentOption[] = [
  {
    runtime: "claude",
    label: "Claude Code",
    defaultModel: "sonnet",
    models: [
      { id: "fable", label: "Fable" },
      { id: "opus", label: "Opus" },
      { id: "sonnet", label: "Sonnet" },
      { id: "haiku", label: "Haiku" },
    ],
  },
  {
    runtime: "codex",
    label: "Codex",
    defaultModel: "codex",
    models: [{ id: "codex", label: "Padrão do Codex" }],
    customModel: true,
  },
];

/** Quando o revisor roda: segundo a Ação (só com risco), sempre ou nunca. */
export type ReviewWhen = "auto" | "always" | "never";

export const reviewWhens: readonly { id: ReviewWhen; label: string }[] = [
  { id: "auto", label: "Só com risco" },
  { id: "always", label: "Sempre" },
  { id: "never", label: "Nunca" },
];

export interface AgentChoice {
  /** Quem implementa e o modelo dele. */
  runtime: Runtime;
  model: string;
  /** Quem revisa; `reviewerModel` vazio = automático (Claude Code, um modelo acima). */
  reviewer: Runtime;
  reviewerModel: string;
  reviewWhen: ReviewWhen;
}

/** Padrão: Claude Code com Sonnet implementa; o próprio Claude Code revisa quando há risco. */
export const defaultChoice = (): AgentChoice => ({
  runtime: "claude",
  model: "sonnet",
  reviewer: "claude",
  reviewerModel: "",
  reviewWhen: "auto",
});

export const agentOf = (runtime: Runtime) => agents.find((agent) => agent.runtime === runtime);

const MODEL_ID = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$/;

/** Rótulo de um modelo; id desconhecido (digitado) aparece como veio. */
export function modelName(runtime: Runtime, model: string) {
  if (!model) return runtime === "claude" ? "um modelo acima" : "padrão";
  return agentOf(runtime)?.models.find((item) => item.id === model)?.label ?? model;
}

/** Modelo aceito para o agente, ou o padrão dele. `blank` permite vazio (revisor automático). */
function normalizeModel(runtime: Runtime, raw: unknown, blank: boolean): string {
  const agent = agentOf(runtime);
  const id = typeof raw === "string" ? raw.trim() : "";
  if (!agent) return "";
  if (!id) return blank && runtime === "claude" ? "" : agent.defaultModel;
  if (agent.models.some((model) => model.id === id)) return id;
  return agent.customModel && MODEL_ID.test(id) ? id : agent.defaultModel;
}

const isRuntime = (value: unknown): value is Runtime => agents.some((a) => a.runtime === value);

/** Aceita qualquer valor lido do armazenamento e devolve uma escolha executável. */
export function normalizeChoice(raw: unknown): AgentChoice {
  const value = (raw ?? {}) as Partial<Record<keyof AgentChoice, unknown>>;
  const runtime = isRuntime(value.runtime) ? value.runtime : "claude";
  const reviewer = isRuntime(value.reviewer) ? value.reviewer : "claude";
  return {
    runtime,
    model: normalizeModel(runtime, value.model, false),
    reviewer,
    reviewerModel: normalizeModel(reviewer, value.reviewerModel, true),
    reviewWhen: reviewWhens.some((item) => item.id === value.reviewWhen)
      ? (value.reviewWhen as ReviewWhen)
      : "auto",
  };
}

/** Agente e modelo em texto; o padrão do Codex e o revisor automático não repetem o modelo. */
function who(runtime: Runtime, model: string) {
  const label = agentOf(runtime)?.label ?? runtime;
  if (runtime === "codex") return model && model !== "codex" ? `${label} · ${model}` : label;
  return model ? `${label} · ${modelName(runtime, model)}` : `${label}, um modelo acima`;
}

/** Resumo de uma linha: quem implementa e quem revisa, e quando. */
export function choiceSummary(choice: AgentChoice) {
  const impl = who(choice.runtime, choice.model);
  if (choice.reviewWhen === "never") return `${impl} · sem revisão`;
  const when = choice.reviewWhen === "always" ? "sempre" : "com risco";
  return `${impl} · revisa: ${who(choice.reviewer, choice.reviewerModel)} (${when})`;
}

const STORAGE_KEY = "harnessy.agent-choice";

/** Preferência por dispositivo; sem armazenamento, vale o padrão. */
export function loadChoice(): AgentChoice {
  try {
    return normalizeChoice(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null"));
  } catch {
    return defaultChoice();
  }
}

export function saveChoice(choice: AgentChoice) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choice));
  } catch {
    // A escolha vale nesta sessão mesmo sem persistir.
  }
}
