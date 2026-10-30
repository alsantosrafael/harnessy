import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentChoice } from "./agents";

export interface AppInfo {
  version: string;
  platform: string;
  agentExecutionAvailable: boolean;
}

export async function getAppInfo(): Promise<AppInfo | null> {
  if (!isTauri()) return null;
  return invoke<AppInfo>("app_info");
}

export interface Workspace {
  id: number;
  name: string;
  folder: string;
  /** `none` é o único modo implementado (ADR 0001). */
  isolation: string;
}
export type TaskStatus = "todo" | "doing" | "review" | "done";
export interface LocalTask {
  id: number;
  workspaceId: number;
  title: string;
  description: string;
  criteria: string;
  /** Protege edições de conteúdo; mover o card não a altera. */
  revision: number;
  status: TaskStatus;
  /** Tarefa de origem quando esta é um passo de "Dividir em passos". */
  parentId: number | null;
  /** Passo anterior da cadeia; a run pode continuar da branch dele. */
  afterId: number | null;
}
export type TaskActor = "user" | "system";
export interface TaskEvent {
  seq: number;
  at: number;
  fromStatus: TaskStatus | null;
  toStatus: TaskStatus;
  actor: TaskActor;
  note: string;
}
/** Payload mínimo de `task-changed`: basta para aplicar o patch sem recarregar a lista. */
export interface TaskChange {
  taskId: number;
  status: TaskStatus;
  revision: number;
  event: TaskEvent;
}
export interface TaskInput {
  id: number | null;
  workspaceId: number;
  revision: number | null;
  title: string;
  description: string;
  criteria: string;
}

function localCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri())
    return Promise.reject(new Error("Abra o aplicativo desktop para acessar os dados locais."));
  return invoke<T>(command, args);
}
export const listWorkspaces = () => localCall<Workspace[]>("list_workspaces");
export const createWorkspace = (name: string, folder: string) =>
  localCall<Workspace>("create_workspace", { name, folder });
export const listTasks = (workspaceId: number) =>
  localCall<LocalTask[]>("list_tasks", { workspaceId });
export const saveTask = (input: TaskInput) => localCall<LocalTask>("save_task", { input });
export const moveTask = (taskId: number, to: TaskStatus) =>
  localCall<void>("move_task", { taskId, to });
export const taskHistory = (taskId: number) => localCall<TaskEvent[]>("task_history", { taskId });

export type Provider = "github" | "codex" | "claude";
export type ConnectionState = "connected" | "expired" | "unavailable" | "unknown";
export interface ConnectionStatus {
  provider: Provider;
  state: ConnectionState;
  detail: string;
  account: string | null;
  binary: string | null;
  loginCommand: string;
  installHint: string;
}
/** Verifica as sessões das CLIs oficiais. Nenhuma credencial chega à interface. */
export const checkConnections = () => localCall<ConnectionStatus[]>("check_connections");

export interface Run {
  id: number;
  workspaceId: number;
  taskId: number;
  title: string;
  action: string;
  role: string;
  runtime: string;
  state: string;
  baseSha: string | null;
  branch: string | null;
  worktreePath: string | null;
  startedAt: number;
  finishedAt: number | null;
  summary: string;
  /** Por que a run ficou presa, deduzido de fatos (falha, nada mudou, ações negadas); vazio = sem bloqueio. */
  blocked: string;
  /** Modelo pedido na run. */
  model: string | null;
  /** Ledger: ciclos do agente e totais; `null` quando algum ciclo não informou o campo. */
  cycles: number | null;
  turns: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
  cacheReadTokens: number | null;
  cacheWriteTokens: number | null;
  costUsd: number | null;
  agentMs: number | null;
  checksMs: number | null;
  /** Veredito da contraprova (`proved`, `not_proved`, `no_tests`, `skipped`, `inconclusive`); `null` = não rodou. */
  proof: string | null;
  /** Esforço de raciocínio pedido pela Ação; `null` quando não definido. */
  effort: string | null;
  /** Modelo do último ciclo gravado. */
  lastModel: string | null;
  /** Próximo modelo da lista acima do último ciclo; `null` no topo. */
  nextModel: string | null;
  /** Veredito do revisor (`approve`, `request_changes`, `invalid`); `null` = não revisou. */
  review: string | null;
  /** Revisor escolhido (`claude` ou `codex`); `null` em runs antigas, que valem `claude`. */
  reviewer: string | null;
  /** Modelo do revisor; vazio ou `null` = automático (um degrau acima). */
  reviewerModel: string | null;
  /** Quando revisar (`auto`, `always`, `never`); `null` vale `auto`. */
  reviewWhen: string | null;
  /** Achados do revisor descartados por não apontarem para o diff. */
  reviewDiscarded: number | null;
  /** Run cuja branch serviu de base (passo encadeado). */
  baseRunId: number | null;
}
export interface RunEvent {
  runId: number;
  seq: number;
  kind: string;
  text: string;
}

/** Ação da tarefa: arquivo TOML embutido ou do usuário. `erro` existe quando ela não pode ser usada. */
export interface ActionInfo {
  id: string;
  nome: string;
  descricao: string;
  /** `null` quando a Ação é inválida e o acesso não pôde ser lido. */
  acesso: "leitura" | "escrita" | null;
  /** `run`: roda sobre o resultado de uma run (revisor), nunca como botão da tarefa. */
  alvo: "tarefa" | "run" | null;
  /** `passos`: a resposta vira proposta de tarefas encadeadas. */
  saida: "texto" | "revisao" | "passos" | null;
  origem: "embutida" | "usuario" | "repositorio";
  /** Ação do repositório nova ou alterada: só roda depois de confirmada neste computador. */
  pendente: boolean;
  erro?: string;
}
/** Ação nova criada pelo formulário; vira `<dados do app>/actions/<id>.toml`. */
export interface NewAction {
  id: string;
  nome: string;
  descricao: string;
  acesso: "leitura" | "escrita";
  modelo: string | null;
  pedido: string;
}

/** Id da Ação embutida que o botão principal "Implementar" dispara. */
export const IMPLEMENT_ACTION = "implementar";

/** Texto exato que o agente receberá para a Ação. */
export const previewPrompt = (workspaceId: number, taskId: number, action = IMPLEMENT_ACTION) =>
  localCall<string>("preview_prompt", { workspaceId, taskId, action });
export const startRun = (
  workspaceId: number,
  taskId: number,
  choice: AgentChoice,
  action = IMPLEMENT_ACTION,
  continueFrom: number | null = null,
) =>
  localCall<Run>("start_run", {
    workspaceId,
    taskId,
    runtime: choice.runtime,
    model: choice.model,
    action,
    continueFrom,
    reviewer: choice.reviewer,
    reviewerModel: choice.reviewerModel,
    reviewWhen: choice.reviewWhen,
  });
/** Passo proposto por "Dividir em passos", editável antes de virar tarefa. */
export interface StepProposal {
  title: string;
  criteria: string;
}
/** Cria os passos confirmados como tarefas encadeadas da tarefa de origem. */
export const createSteps = (workspaceId: number, parentId: number, steps: StepProposal[]) =>
  localCall<LocalTask[]>("create_steps", { workspaceId, parentId, steps });
/** Ações disponíveis; com `workspaceId`, confere que o workspace existe. */
export const listActions = (workspaceId?: number) =>
  localCall<ActionInfo[]>("list_actions", { workspaceId });
/** Confirma o conteúdo atual de uma Ação versionada no repositório do workspace. */
export const approveAction = (workspaceId: number, id: string) =>
  localCall<void>("approve_action", { workspaceId, id });
/** Cria uma Ação do usuário; o núcleo valida com o mesmo parser dos arquivos. */
export const createAction = (input: NewAction) => localCall<ActionInfo>("create_action", { input });
/** Dá o bloqueio de uma run por resolvido; fica registrado no histórico da tarefa. */
export const resolveBlock = (runId: number) => localCall<void>("resolve_block", { runId });
export const cancelRun = (runId: number) => localCall<void>("cancel_run", { runId });
/** Ciclo extra numa run que falhou, com o próximo modelo da lista (decidido pelo núcleo). */
export const continueRun = (runId: number) => localCall<Run>("continue_run", { runId });
export const listRuns = (workspaceId: number) => localCall<Run[]>("list_runs", { workspaceId });
export const listRunEvents = (runId: number, afterSeq: number) =>
  localCall<RunEvent[]>("list_run_events", { runId, afterSeq });

/** Assina um evento do núcleo; devolve a função que cancela a assinatura. */
async function subscribe<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<T>(name, (event) => handler(event.payload));
}
export const onRunEvent = (handler: (event: RunEvent) => void) =>
  subscribe<RunEvent>("run-event", handler);
export const onRunState = (handler: (run: Run) => void) => subscribe<Run>("run-state", handler);
export const onTaskChanged = (handler: (change: TaskChange) => void) =>
  subscribe<TaskChange>("task-changed", handler);
