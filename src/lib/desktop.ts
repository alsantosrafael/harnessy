import { invoke, isTauri } from "@tauri-apps/api/core";

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
}
export interface LocalTask {
  id: number;
  workspaceId: number;
  title: string;
  description: string;
  criteria: string;
  revision: number;
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
