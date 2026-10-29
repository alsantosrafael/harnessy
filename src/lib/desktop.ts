import { invoke, isTauri } from '@tauri-apps/api/core';

export interface AppInfo {
  version: string;
  platform: string;
  agentExecutionAvailable: boolean;
}

export async function getAppInfo(): Promise<AppInfo | null> {
  if (!isTauri()) return null;
  return invoke<AppInfo>('app_info');
}
