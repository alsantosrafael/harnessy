import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Plus } from "lucide-react";
import { applyChange, applyMove, blockedByTask, groupByStatus, statusName } from "../lib/board";
import {
  listRunEvents,
  listRuns,
  listTasks,
  moveTask,
  resolveBlock,
  onRunEvent,
  onRunState,
  onTaskChanged,
  startRun,
  type LocalTask,
  type Run,
  type RunEvent,
  type TaskStatus,
  type Workspace,
  saveTask,
} from "../lib/desktop";
import type { AgentChoice } from "../lib/agents";
import { canStartRun, chainBase, isActive, mergeEvents, upsertRun } from "../lib/runs";
import { Kanban, KanbanSkeleton } from "./Kanban";
import { RunsPanel } from "./RunsPanel";
import { TaskDetail } from "./TaskDetail";
import { WorkspaceForm } from "./WorkspaceForm";
import "../styles/local-work.css";
import "../styles/run-board.css";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const FLASH_MS = 700;
const UNDO_MS = 5_000;
/** Rede de segurança: se um evento ao vivo se perder, o banco local corrige a tela. */
const RUN_POLL_MS = 2_000;

interface Notice {
  text: string;
  undo?: () => void;
}

interface Props {
  hidden: boolean;
  agentReady: boolean;
  onConnect: () => void;
  /** Workspaces e seleção vivem no App, compartilhados com o chat. */
  workspaces: Workspace[];
  workspacesReady: boolean;
  workspaceId: number | null;
  loadError: string;
  onWorkspaceChange: (id: number) => void;
  onWorkspaceCreated: (workspace: Workspace) => void;
  /** Muda quando algo fora do quadro (o chat) criou tarefa ou run. */
  refreshKey: number;
  choice: AgentChoice;
  onChoice: (choice: AgentChoice) => void;
}

export function BoardPanel({
  hidden,
  agentReady,
  onConnect,
  workspaces,
  workspacesReady,
  workspaceId,
  loadError,
  onWorkspaceChange,
  onWorkspaceCreated,
  refreshKey,
  choice,
  onChoice,
}: Props) {
  const [creatingWorkspace, setCreatingWorkspace] = useState(false);
  const [tasks, setTasks] = useState<LocalTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [flashId, setFlashId] = useState<number | null>(null);
  const [historyTick, setHistoryTick] = useState(0);
  const [runs, setRuns] = useState<Run[]>([]);
  const [openRun, setOpenRun] = useState<number | null>(null);
  const [events, setEvents] = useState<RunEvent[]>([]);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState<Notice | null>(null);
  const [live, setLive] = useState("");

  // Refs deixam `move`/`select` estáveis, para os cards memoizados não re-renderizarem.
  const tasksRef = useRef(tasks);
  tasksRef.current = tasks;
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  const openRunRef = useRef(openRun);
  openRunRef.current = openRun;
  const runsRef = useRef(runs);
  runsRef.current = runs;
  const dirtyRef = useRef(false);
  const timers = useRef<{ flash?: number; notice?: number }>({});
  const lastSeq = events.reduce((max, event) => Math.max(max, event.seq), 0);
  const lastSeqRef = useRef(0);
  lastSeqRef.current = lastSeq;

  const groups = useMemo(() => groupByStatus(tasks), [tasks]);
  const working = useMemo(() => new Set(runs.filter(isActive).map((run) => run.taskId)), [runs]);
  const blocked = useMemo(() => blockedByTask(runs, tasks), [runs, tasks]);
  const selected = tasks.find((task) => task.id === selectedId) ?? null;

  const flash = useCallback((id: number) => {
    setFlashId(id);
    window.clearTimeout(timers.current.flash);
    timers.current.flash = window.setTimeout(() => setFlashId(null), FLASH_MS);
  }, []);

  const say = useCallback((next: Notice) => {
    setNotice(next);
    setLive(next.text);
    window.clearTimeout(timers.current.notice);
    timers.current.notice = window.setTimeout(() => setNotice(null), UNDO_MS);
  }, []);

  useEffect(
    () => () => {
      window.clearTimeout(timers.current.flash);
      window.clearTimeout(timers.current.notice);
    },
    [],
  );

  // Otimista: a tela muda na hora; o núcleo confirma (ou o card volta e o erro aparece).
  const move = useCallback(
    (id: number, to: TaskStatus, undoable = true) => {
      const task = tasksRef.current.find((item) => item.id === id);
      if (!task || task.status === to) return;
      const from = task.status;
      setTasks((current) => applyMove(current, id, to));
      flash(id);
      moveTask(id, to)
        .then(() =>
          say({
            text: `"${task.title}" movida para ${statusName(to)}.`,
            undo: undoable ? () => move(id, from, false) : undefined,
          }),
        )
        .catch((cause) => {
          setTasks((current) => applyMove(current, id, from));
          setError(message(cause));
        });
    },
    [flash, say],
  );

  const select = useCallback((id: number | null) => {
    if (dirtyRef.current && !window.confirm("Descartar as alterações não salvas desta tarefa?")) {
      return;
    }
    dirtyRef.current = false;
    setSelectedId(id);
  }, []);
  const setDirty = useCallback((dirty: boolean) => {
    dirtyRef.current = dirty;
  }, []);

  // Eventos do núcleo valem mesmo com o painel escondido.
  useEffect(() => {
    const stops = [
      onRunState((run) => setRuns((current) => upsertRun(current, run))),
      onRunEvent((event) => {
        if (event.runId === openRunRef.current) setEvents((c) => mergeEvents(c, [event]));
      }),
      onTaskChanged((change) => {
        setTasks((current) => applyChange(current, change));
        if (change.taskId === selectedRef.current) setHistoryTick((tick) => tick + 1);
      }),
    ];
    return () => {
      for (const stop of stops) stop.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (loadError) setError(loadError);
  }, [loadError]);

  // Trocar de workspace zera a seleção; recarregar por `refreshKey` não.
  useEffect(() => {
    setLoading(workspaceId !== null || !workspacesReady);
    setTasks([]);
    setRuns([]);
    setSelectedId(null);
    setOpenRun(null);
  }, [workspaceId, workspacesReady]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `refreshKey` é o gatilho de recarga vindo do chat.
  useEffect(() => {
    if (workspaceId === null) return;
    let alive = true;
    Promise.all([listTasks(workspaceId), listRuns(workspaceId)])
      .then(([taskRows, runRows]) => {
        if (!alive) return;
        setTasks(taskRows);
        setRuns(runRows);
      })
      .catch((cause) => alive && setError(message(cause)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [workspaceId, refreshKey]);

  useEffect(() => {
    if (openRun === null) return;
    let alive = true;
    setEvents([]);
    listRunEvents(openRun, 0)
      .then((rows) => alive && setEvents((current) => mergeEvents(current, rows)))
      .catch((cause) => alive && setError(message(cause)));
    return () => {
      alive = false;
    };
  }, [openRun]);

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

  // `n` leva ao campo de nova tarefa, como em qualquer lista com atalhos.
  useEffect(() => {
    if (hidden) return;
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const typing = ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
      if (event.key !== "n" || typing || event.metaKey || event.ctrlKey || event.altKey) return;
      event.preventDefault();
      document.getElementById("new-task-title")?.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [hidden]);

  const create = useCallback(
    async (title: string) => {
      if (workspaceId === null) return;
      try {
        const task = await saveTask({
          workspaceId,
          id: null,
          revision: null,
          title,
          description: "",
          criteria: "",
        });
        setTasks((current) => [task, ...current]);
        flash(task.id);
        setLive(`Tarefa "${task.title}" criada em A fazer.`);
      } catch (cause) {
        setError(message(cause));
      }
    },
    [workspaceId, flash],
  );

  async function start(taskId: number, action?: string, continueFrom: number | null = null) {
    if (workspaceId === null) return;
    setStarting(true);
    setError("");
    try {
      const run = await startRun(
        workspaceId,
        taskId,
        choice.runtime,
        choice.model,
        action,
        continueFrom,
        choice.reviewer,
      );
      setRuns((current) => upsertRun(current, run));
      setOpenRun(run.id);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setStarting(false);
    }
  }

  async function resolve(runId: number) {
    try {
      await resolveBlock(runId);
    } catch (cause) {
      setError(message(cause));
    }
  }

  const workspace = workspaces.find((item) => item.id === workspaceId) ?? null;
  const startable = canStartRun(agentReady, runs);

  return (
    <section id="board-panel" className="local-work" aria-label="Tarefas" hidden={hidden}>
      <header className="page-header">
        <h1>Tarefas</h1>
        {workspaces.length > 0 && (
          <label className="workspace-picker">
            <span className="sr-only">Workspace</span>
            <select
              value={workspaceId ?? ""}
              onChange={(event) => {
                if (!dirtyRef.current || window.confirm("Descartar as alterações não salvas?")) {
                  dirtyRef.current = false;
                  onWorkspaceChange(Number(event.target.value));
                }
              }}
            >
              {workspaces.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
        )}
        {workspace && (
          <span className="local-folder" title={workspace.folder}>
            {workspace.folder}
          </span>
        )}
        <div className="page-actions">
          <details className="shortcuts">
            <summary>Atalhos</summary>
            <p>
              <kbd>n</kbd> nova tarefa · <kbd>]</kbd> avança · <kbd>[</kbd> volta
            </p>
          </details>
          <button
            type="button"
            className="button"
            aria-expanded={creatingWorkspace}
            onClick={() => setCreatingWorkspace(!creatingWorkspace)}
          >
            <Plus size={14} aria-hidden="true" />
            Workspace
          </button>
        </div>
      </header>
      {!agentReady && (
        <p className="local-error" role="status">
          Claude Code ou git não foram encontrados. Verifique em{" "}
          <button type="button" className="link-button" onClick={onConnect}>
            Conexões
          </button>
          .
        </p>
      )}
      {error && (
        <p className="local-error" role="alert">
          {error}{" "}
          <button type="button" className="link-button" onClick={() => setError("")}>
            Dispensar
          </button>
        </p>
      )}
      {creatingWorkspace && (
        <WorkspaceForm
          onCreated={(created) => {
            onWorkspaceCreated(created);
            setCreatingWorkspace(false);
            say({ text: "Workspace criado e salvo neste dispositivo." });
          }}
        />
      )}
      {workspacesReady && workspaces.length === 0 && !creatingWorkspace && (
        <div className="local-empty">
          <h3>Comece pelo seu projeto.</h3>
          <p>Crie um workspace para guardar tarefas. GitHub não é necessário.</p>
          <button
            type="button"
            className="button primary"
            onClick={() => setCreatingWorkspace(true)}
          >
            Criar primeiro workspace
          </button>
        </div>
      )}
      {workspaces.length > 0 && (
        <div className={`board-layout${selected ? " has-detail" : ""}`}>
          <div className="board-main">
            {loading ? (
              <KanbanSkeleton />
            ) : (
              <Kanban
                groups={groups}
                selectedId={selectedId}
                working={working}
                blocked={blocked}
                flashId={flashId}
                onSelect={select}
                onMove={move}
                onCreate={create}
              />
            )}
            <p className="sr-only" role="status" aria-live="polite">
              {live}
            </p>
            {notice && (
              <p className="toast" role="presentation">
                <span aria-hidden="true">✓</span> {notice.text}
                {notice.undo && (
                  <button
                    type="button"
                    className="link-button"
                    onClick={() => {
                      notice.undo?.();
                      setNotice(null);
                    }}
                  >
                    Desfazer
                  </button>
                )}
              </p>
            )}
            <RunsPanel
              runs={runs}
              openRun={openRun}
              events={events}
              onOpen={setOpenRun}
              onError={setError}
              onStepsCreated={(created) =>
                setTasks((current) => [...created].reverse().concat(current))
              }
            />
          </div>
          {selected && (
            <aside className="detail-drawer" aria-label={`Tarefa #${selected.id}`}>
              <TaskDetail
                key={selected.id}
                task={selected}
                historyTick={historyTick}
                block={blocked.get(selected.id)}
                chainFrom={chainBase(selected, runs)}
                previousTitle={
                  tasks.find((item) => item.id === selected.afterId)?.title ?? "passo anterior"
                }
                choice={choice}
                onChoice={onChoice}
                canStart={startable}
                starting={starting}
                onMove={move}
                onSaved={(saved) =>
                  setTasks((current) =>
                    current.map((item) => (item.id === saved.id ? saved : item)),
                  )
                }
                onStart={start}
                onResolve={resolve}
                onDirty={setDirty}
                onClose={() => select(null)}
              />
              {!startable && agentReady && (
                <p className="muted">Já existe uma run ativa neste workspace.</p>
              )}
            </aside>
          )}
        </div>
      )}
    </section>
  );
}
