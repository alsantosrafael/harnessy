import type { PreviewInfo } from "./desktop";

/** Linhas de log mantidas na tela; o núcleo guarda 500, a tela mostra o rabo. */
export const MAX_LOG_LINES = 300;

export const isLive = (info: PreviewInfo | null | undefined) =>
  info?.state === "starting" || info?.state === "running";

export interface PreviewLabel {
  text: string;
  tone: "info" | "success" | "critical" | "neutral";
}

/** Estado do ambiente em palavras, para o selo e para quem usa leitor de tela. */
export function previewLabel(info: PreviewInfo | null | undefined): PreviewLabel {
  switch (info?.state) {
    case "starting":
      return { text: "Subindo…", tone: "info" };
    case "running":
      return { text: "No ar", tone: "success" };
    case "failed":
      return {
        text: info.exitCode === null ? "Caiu" : `Caiu (código ${info.exitCode})`,
        tone: "critical",
      };
    case "stopped":
      return { text: "Parado", tone: "neutral" };
    default:
      return { text: "Desligado", tone: "neutral" };
  }
}

/** Troca o estado de uma run, mantendo as demais. */
export function upsertPreview(
  current: Record<number, PreviewInfo>,
  info: PreviewInfo,
): Record<number, PreviewInfo> {
  return { ...current, [info.runId]: info };
}

export function appendLog(lines: string[], line: string, max = MAX_LOG_LINES): string[] {
  const next = [...lines, line];
  return next.length > max ? next.slice(next.length - max) : next;
}
