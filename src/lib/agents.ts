import type { Runtime } from "./roles";

export interface ModelOption {
  id: string;
  label: string;
}

export interface AgentOption {
  runtime: Runtime;
  label: string;
  models: readonly ModelOption[];
  defaultModel: string;
  /** Presente quando o agente ainda não executa; é o motivo mostrado ao usuário. */
  unavailable?: string;
}

/**
 * Agentes e modelos que o Implementador pode usar (o Codex revisa, não implementa). Os ids de modelo são aliases do
 * `claude --model` e precisam estar também em `MODELS`, em `src-tauri/src/run/guard.rs`,
 * que é quem valida a escolha.
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
    defaultModel: "",
    models: [],
    unavailable: "só revisa, por enquanto",
  },
];

/** Quem revisa o resultado de uma run de escrita. */
export type Reviewer = "claude" | "codex";

export const reviewers: readonly { id: Reviewer; label: string }[] = [
  { id: "claude", label: "Claude Code, um modelo acima" },
  { id: "codex", label: "Codex" },
];

export interface AgentChoice {
  runtime: Runtime;
  model: string;
  reviewer: Reviewer;
}

/** Padrão: Claude Code com Sonnet implementa e o próprio Claude Code revisa. */
export const defaultChoice = (): AgentChoice => ({
  runtime: "claude",
  model: "sonnet",
  reviewer: "claude",
});

export const agentOf = (runtime: Runtime) => agents.find((agent) => agent.runtime === runtime);

export const modelLabel = (choice: AgentChoice) =>
  agentOf(choice.runtime)?.models.find((model) => model.id === choice.model)?.label ?? choice.model;

/** Aceita qualquer valor lido do armazenamento e devolve uma escolha executável. */
export function normalizeChoice(raw: unknown): AgentChoice {
  const value = (raw ?? {}) as Partial<AgentChoice>;
  const reviewer = reviewers.some((item) => item.id === value.reviewer)
    ? (value.reviewer as Reviewer)
    : "claude";
  const agent = agents.find((item) => item.runtime === value.runtime);
  if (!agent || agent.unavailable) return { ...defaultChoice(), reviewer };
  const known = agent.models.some((model) => model.id === value.model);
  return {
    runtime: agent.runtime,
    model: known ? String(value.model) : agent.defaultModel,
    reviewer,
  };
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
