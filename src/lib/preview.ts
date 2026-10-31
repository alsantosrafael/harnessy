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

const PORT_BUSY = /already in use|EADDRINUSE|address already in use|porta .* em uso/i;

/**
 * Frase de ajuda quando o ambiente caiu por um motivo que o usuário resolve no comando,
 * deduzida do fim do log; `null` quando não há uma dica segura.
 */
export function hintFor(
  info: PreviewInfo | null | undefined,
  log: readonly string[],
): string | null {
  if (info?.state !== "failed") return null;
  if (log.slice(-30).some((line) => PORT_BUSY.test(line))) {
    return "A porta está ocupada por outro programa (pode ser o próprio Harnessy em desenvolvimento). Escreva {port} no comando, no lugar do número da porta, e o Harnessy escolhe uma livre.";
  }
  return null;
}

export type LogKind = "error" | "warn" | "ok" | "info" | "note" | "command" | "dim" | "plain";

const ERROR_LINE =
  /\b(error|erro|failed|failure|fatal|panic|exception|ERR!|EADDRINUSE|ELIFECYCLE)\b|exited with code [1-9]|^caiu\b/i;
const WARN_LINE = /\b(warn|warning|deprecated|aviso)\b/i;
const OK_LINE = /(✓|✔)|\b(ready in|built in|compiled|listening on|server running)\b/i;
const URL_LINE = /https?:\/\/(localhost|127\.0\.0\.1|\[::1\]):\d+/i;
// O que o próprio Harnessy conta durante a preparação da cópia.
const NOTE_LINE =
  /^(A cópia desta run|Preparando|Instalando|Cópia pronta|O ambiente|Não foi possível iniciar)/;

/**
 * Tipo de uma linha do log, para colorir o terminal. O erro vence os demais (uma linha que diz
 * "error" nunca vira "ok"), e o que não casa com nada fica sem cor.
 */
export function logKind(line: string): LogKind {
  const text = line.trim();
  if (text === "") return "plain";
  if (text.startsWith("$ ")) return "command";
  if (NOTE_LINE.test(text)) return "note";
  if (ERROR_LINE.test(text)) return "error";
  if (WARN_LINE.test(text)) return "warn";
  if (OK_LINE.test(text)) return "ok";
  if (URL_LINE.test(text)) return "info";
  if (/^\s*(at |\d+ \|)/.test(line) || text.startsWith("└") || text.startsWith("├")) return "dim";
  return "plain";
}
