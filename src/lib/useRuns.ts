import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AgentChoice } from "./agents";
import {
  listRunEvents,
  listRuns,
  onRunEvent,
  onRunState,
  type Run,
  type RunEvent,
  startRun,
} from "./desktop";
import { isActive, mergeEvents, threadOf, upsertRun } from "./runs";

/** Rede de segurança: se um evento ao vivo se perder, o banco local corrige a tela. */
const RUN_POLL_MS = 2_000;
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export interface RunsState {
  runs: Run[];
  openRun: number | null;
  /** Runs da tarefa da run aberta, da mais antiga à mais nova: a conversa inteira. */
  thread: Run[];
  eventsByRun: Record<number, RunEvent[]>;
  starting: boolean;
  open: (id: number | null) => void;
  /** Traz o log de uma run ainda não carregado (por exemplo, para ancorar achados no diff). */
  ensureEvents: (runId: number) => void;
  /** Esquece as runs de uma tarefa excluída. */
  forgetTask: (taskId: number) => void;
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
 * Runs do workspace e o log da conversa aberta, compartilhados entre o quadro e o chat.
 * A conversa de uma tarefa reúne todas as runs dela, então pedir um ajuste não apaga o
 * que já foi dito. Eventos do núcleo valem mesmo com a tela escondida.
 */
export function useRuns(
  workspaceId: number | null,
  refreshKey: number,
  onError: (text: string) => void,
): RunsState {
  const [runs, setRuns] = useState<Run[]>([]);
  const [openRun, setOpenRun] = useState<number | null>(null);
  const [eventsByRun, setEventsByRun] = useState<Record<number, RunEvent[]>>({});
  const [starting, setStarting] = useState(false);
  const openRef = useRef(openRun);
  openRef.current = openRun;
  const runsRef = useRef(runs);
  runsRef.current = runs;
  const eventsRef = useRef(eventsByRun);
  eventsRef.current = eventsByRun;
  // Runs cujo log já foi pedido ao banco, para não repetir a carga.
  const loaded = useRef(new Set<number>());

  const taskOfOpen = runs.find((run) => run.id === openRun)?.taskId ?? null;
  const thread = useMemo(() => threadOf(runs, taskOfOpen), [runs, taskOfOpen]);

  const addEvents = useCallback((runId: number, incoming: RunEvent[]) => {
    setEventsByRun((current) => ({
      ...current,
      [runId]: mergeEvents(current[runId] ?? [], incoming),
    }));
  }, []);

  useEffect(() => {
    const stops = [
      onRunState((run) => setRuns((current) => upsertRun(current, run))),
      onRunEvent((event) => addEvents(event.runId, [event])),
    ];
    return () => {
      for (const stop of stops) stop.then((off) => off());
    };
  }, [addEvents]);

  // Trocar de workspace zera a seleção; recarregar por `refreshKey` não.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `workspaceId` é o gatilho do reset.
  useEffect(() => {
    setRuns([]);
    setOpenRun(null);
    setEventsByRun({});
    loaded.current = new Set();
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

  // Carrega o log de cada run da conversa aberta que ainda não veio do banco.
  const threadKey = thread.map((run) => run.id).join(",");
  // biome-ignore lint/correctness/useExhaustiveDependencies: `threadKey` resume as runs da conversa.
  useEffect(() => {
    for (const run of thread) {
      if (loaded.current.has(run.id)) continue;
      loaded.current.add(run.id);
      listRunEvents(run.id, 0)
        .then((rows) => addEvents(run.id, rows))
        .catch((cause) => {
          loaded.current.delete(run.id);
          onError(message(cause));
        });
    }
  }, [threadKey, addEvents, onError]);

  const activeKey = thread
    .filter(isActive)
    .map((run) => run.id)
    .join(",");
  useEffect(() => {
    if (workspaceId === null || !activeKey) return;
    const ids = activeKey.split(",").map(Number);
    const timer = setInterval(() => {
      for (const id of ids) {
        const last = (eventsRef.current[id] ?? []).reduce((max, e) => Math.max(max, e.seq), 0);
        listRunEvents(id, last)
          .then((rows) => rows.length > 0 && addEvents(id, rows))
          .catch(() => {});
      }
      listRuns(workspaceId)
        .then(setRuns)
        .catch(() => {});
    }, RUN_POLL_MS);
    return () => clearInterval(timer);
  }, [workspaceId, activeKey, addEvents]);

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

  const forgetTask = useCallback((taskId: number) => {
    const open = runsRef.current.find((run) => run.id === openRef.current);
    if (open?.taskId === taskId) setOpenRun(null);
    // O SQLite reaproveita ids: sem limpar o cache, uma run nova herdaria a conversa da excluída.
    const gone = runsRef.current.filter((run) => run.taskId === taskId).map((run) => run.id);
    for (const id of gone) loaded.current.delete(id);
    setEventsByRun((current) => {
      const next = { ...current };
      for (const id of gone) delete next[id];
      return next;
    });
    setRuns((current) => current.filter((run) => run.taskId !== taskId));
  }, []);

  const ensureEvents = useCallback(
    (runId: number) => {
      if (loaded.current.has(runId)) return;
      loaded.current.add(runId);
      listRunEvents(runId, 0)
        .then((rows) => addEvents(runId, rows))
        .catch(() => loaded.current.delete(runId));
    },
    [addEvents],
  );

  return {
    runs,
    openRun,
    thread,
    eventsByRun,
    starting,
    open: setOpenRun,
    ensureEvents,
    forgetTask,
    start,
  };
}
