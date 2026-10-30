import { useCallback, useEffect, useRef, useState } from "react";
import {
  listRunEvents,
  listRuns,
  onRunEvent,
  onRunState,
  startRun,
  type Run,
  type RunEvent,
} from "./desktop";
import type { AgentChoice } from "./agents";
import { isActive, mergeEvents, upsertRun } from "./runs";

/** Rede de segurança: se um evento ao vivo se perder, o banco local corrige a tela. */
const RUN_POLL_MS = 2_000;
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export interface RunsState {
  runs: Run[];
  openRun: number | null;
  events: RunEvent[];
  starting: boolean;
  open: (id: number | null) => void;
  /** Inicia uma run e a abre; devolve `null` (e chama `onError`) se falhar. */
  start: (
    taskId: number,
    choice: AgentChoice,
    action?: string,
    continueFrom?: number | null,
    instruction?: string | null,
  ) => Promise<Run | null>;
}

/**
 * Runs do workspace e o log da aberta, compartilhados entre o quadro e o chat.
 * Eventos do núcleo valem mesmo com a tela escondida.
 */
export function useRuns(
  workspaceId: number | null,
  refreshKey: number,
  onError: (text: string) => void,
): RunsState {
  const [runs, setRuns] = useState<Run[]>([]);
  const [openRun, setOpenRun] = useState<number | null>(null);
  const [events, setEvents] = useState<RunEvent[]>([]);
  const [starting, setStarting] = useState(false);
  const openRef = useRef(openRun);
  openRef.current = openRun;
  const lastSeqRef = useRef(0);
  lastSeqRef.current = events.reduce((max, event) => Math.max(max, event.seq), 0);

  useEffect(() => {
    const stops = [
      onRunState((run) => setRuns((current) => upsertRun(current, run))),
      onRunEvent((event) => {
        if (event.runId === openRef.current) setEvents((c) => mergeEvents(c, [event]));
      }),
    ];
    return () => {
      for (const stop of stops) stop.then((off) => off());
    };
  }, []);

  // Trocar de workspace zera a seleção; recarregar por `refreshKey` não.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `workspaceId` é o gatilho do reset.
  useEffect(() => {
    setRuns([]);
    setOpenRun(null);
  }, [workspaceId]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `refreshKey` é o gatilho de recarga vindo do chat.
  useEffect(() => {
    if (workspaceId === null) return;
    let alive = true;
    listRuns(workspaceId)
      .then((rows) => alive && setRuns(rows))
      .catch((cause) => alive && onError(message(cause)));
    return () => {
      alive = false;
    };
  }, [workspaceId, refreshKey, onError]);

  useEffect(() => {
    if (openRun === null) return;
    let alive = true;
    setEvents([]);
    listRunEvents(openRun, 0)
      .then((rows) => alive && setEvents((current) => mergeEvents(current, rows)))
      .catch((cause) => alive && onError(message(cause)));
    return () => {
      alive = false;
    };
  }, [openRun, onError]);

  const openRunActive = runs.some((run) => run.id === openRun && isActive(run));
  useEffect(() => {
    if (openRun === null || workspaceId === null || !openRunActive) return;
    let alive = true;
    const timer = setInterval(() => {
      listRunEvents(openRun, lastSeqRef.current)
        .then((rows) => alive && rows.length > 0 && setEvents((c) => mergeEvents(c, rows)))
        .catch(() => {});
      listRuns(workspaceId)
        .then((rows) => alive && setRuns(rows))
        .catch(() => {});
    }, RUN_POLL_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [openRun, workspaceId, openRunActive]);

  const start = useCallback<RunsState["start"]>(
    async (taskId, choice, action, continueFrom = null, instruction = null) => {
      if (workspaceId === null) return null;
      setStarting(true);
      try {
        const run = await startRun(workspaceId, taskId, choice, action, continueFrom, instruction);
        setRuns((current) => upsertRun(current, run));
        setOpenRun(run.id);
        return run;
      } catch (cause) {
        onError(message(cause));
        return null;
      } finally {
        setStarting(false);
      }
    },
    [workspaceId, onError],
  );

  return { runs, openRun, events, starting, open: setOpenRun, start };
}
