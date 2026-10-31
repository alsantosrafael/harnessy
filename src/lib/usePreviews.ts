import { useCallback, useEffect, useState } from "react";
import {
  listPreviews,
  onPreviewLog,
  onPreviewState,
  type PreviewInfo,
  previewSnapshot,
} from "./desktop";
import { appendLog, MAX_LOG_LINES, upsertPreview } from "./preview";

export interface PreviewsState {
  infos: Record<number, PreviewInfo>;
  logs: Record<number, string[]>;
  /** Traz estado e log de uma run (a tela pode chegar depois dos eventos). */
  load: (runId: number) => Promise<void>;
}

/** Ambientes de teste do núcleo: estado e log ao vivo, valem mesmo com a tela escondida. */
export function usePreviews(): PreviewsState {
  const [infos, setInfos] = useState<Record<number, PreviewInfo>>({});
  const [logs, setLogs] = useState<Record<number, string[]>>({});

  useEffect(() => {
    listPreviews()
      .then((rows) => {
        for (const info of rows) setInfos((current) => upsertPreview(current, info));
      })
      .catch(() => undefined);
    const stops = [
      onPreviewState((info) => setInfos((current) => upsertPreview(current, info))),
      onPreviewLog(({ runId, line }) =>
        setLogs((current) => ({ ...current, [runId]: appendLog(current[runId] ?? [], line) })),
      ),
    ];
    return () => {
      for (const stop of stops) stop.then((off) => off());
    };
  }, []);

  const load = useCallback(async (runId: number) => {
    const [info, lines] = await previewSnapshot(runId);
    if (info) setInfos((current) => upsertPreview(current, info));
    setLogs((current) => ({ ...current, [runId]: lines.slice(-MAX_LOG_LINES) }));
  }, []);

  return { infos, logs, load };
}
