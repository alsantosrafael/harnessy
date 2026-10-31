import { useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { Plus } from "lucide-react";
import {
  type Attention,
  attentionOf,
  byUrgency,
  notificationFor,
  countPhrase,
  type Filter,
  matchesFilter,
  withAgents,
} from "../lib/attention";
import { applyChange, applyMove, blockedByTask, groupByStatus, statusName } from "../lib/board";
import {
  approveProfile,
  cancelRun,
  deleteTask,
  getProfile,
  listDeliveries,
  listTasks,
  mergeDelivery,
  moveTask,
  openPreview,
  publishRun,
  refreshDelivery,
  saveProfile,
  startPreview,
  stopPreview,
  type Delivery,
  resolveBlock,
  reviewRun,
  onTaskChanged,
  type LocalTask,
  type TaskStatus,
  type Workspace,
  saveTask,
} from "../lib/desktop";
import type { AgentChoice } from "../lib/agents";
import { deliveryOf, needsRefresh, stagesByTask, workRun } from "../lib/delivery";
import { parseReview, type Finding } from "../lib/feed";
import { progressOf } from "../lib/progress";
import { isLive } from "../lib/preview";
import { canStartRun, chainBase, isActive } from "../lib/runs";
import { useAttentionSignals } from "../lib/useAttention";
import { usePreviews } from "../lib/usePreviews";
import type { RunsState } from "../lib/useRuns";
import { nextAction, quickStates, type QuickId } from "../lib/quick";
import { type Confirmation, ConfirmDialog } from "./ConfirmDialog";
import { Kanban, KanbanSkeleton, type NewTaskDraft } from "./Kanban";
import { ChangesPanel } from "./ChangesPanel";
import { PreviewPanel } from "./PreviewPanel";
import { PreviewSetup } from "./PreviewSetup";
import { QuickActions } from "./QuickActions";
import { TaskDetail, TaskDetailSkeleton } from "./TaskDetail";
import { WorkspaceForm } from "./WorkspaceForm";
import "../styles/local-work.css";
import "../styles/run-board.css";

/** Achados da última revisão da run (os que bloqueiam e as sugestões), se o log dela já foi carregado. */
function reviewFindings(events: { kind: string; text: string }[] | undefined): Finding[] {
  const last = [...(events ?? [])].reverse().find((event) => event.kind === "review");
  const review = last ? parseReview(last.text) : null;
  return review ? [...review.blocking, ...review.suggestions] : [];
}

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const FLASH_MS = 700;
const UNDO_MS = 5_000;
/** O GitHub muda por fora (checks, conflitos, merge): reler os PRs abertos de tempos em tempos. */
const PR_POLL_MS = 90_000;

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
  /** Runs compartilhadas com o chat; iniciar uma abre a conversa dela. */
  runs: RunsState;
  onOpenRun: () => void;
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
  runs: runsState,
  onOpenRun,
}: Props) {
  const [creatingWorkspace, setCreatingWorkspace] = useState(false);
  const [tasks, setTasks] = useState<LocalTask[]>([]);
  const [deliveries, setDeliveries] = useState<Delivery[]>([]);
  const [delivering, setDelivering] = useState(false);
  const [filter, setFilter] = useState<Filter>("all");
  const [deliveriesReady, setDeliveriesReady] = useState(false);
  const previews = usePreviews();
  const [setup, setSetup] = useState<{
    runId: number;
    suggestion: string;
    error: string | null;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [flashId, setFlashId] = useState<number | null>(null);
  const [historyTick, setHistoryTick] = useState(0);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState<Notice | null>(null);
  const [live, setLive] = useState("");
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  // Continuar do passo anterior: vale para o botão do detalhe e para os comandos rápidos.
  const [useChain, setUseChain] = useState(true);
  const detailRef = useRef<HTMLElement>(null);

  // Refs deixam `move`/`select` estáveis, para os cards memoizados não re-renderizarem.
  const tasksRef = useRef(tasks);
  const runNextRef = useRef<(() => void) | null>(null);
  const deliveriesRef = useRef(deliveries);
  deliveriesRef.current = deliveries;
  const stagesRef = useRef<ReturnType<typeof stagesByTask>>(new Map());
  tasksRef.current = tasks;
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  const dirtyRef = useRef(false);
  const timers = useRef<{ flash?: number; notice?: number }>({});

  const { runs, starting } = runsState;
  const working = useMemo(() => new Set(runs.filter(isActive).map((run) => run.taskId)), [runs]);
  const blocked = useMemo(() => blockedByTask(runs, tasks), [runs, tasks]);
  const testing = useMemo(() => {
    const ids = new Set<number>();
    for (const task of tasks) {
      const work = workRun(runs, task.id);
      if (work && previews.infos[work.id]?.state === "running") ids.add(task.id);
    }
    return ids;
  }, [tasks, runs, previews.infos]);
  const stages = useMemo(() => stagesByTask(tasks, runs, deliveries), [tasks, runs, deliveries]);
  const attention = useMemo(
    () => attentionOf(tasks, stages, blocked, working),
    [tasks, stages, blocked, working],
  );
  const withAgentsCount = useMemo(
    () => tasks.filter((task) => withAgents(task, stages, working)).length,
    [tasks, stages, working],
  );
  const groups = useMemo(
    () =>
      groupByStatus(
        filter === "all"
          ? tasks
          : tasks.filter((task) => matchesFilter(filter, task, attention, stages, working)),
      ),
    [tasks, filter, attention, stages, working],
  );
  const selected = tasks.find((task) => task.id === selectedId) ?? null;
  // O card destaca no mesmo instante do clique; o detalhe (mais pesado) monta logo depois, atrás de um esqueleto.
  const detailId = useDeferredValue(selectedId);
  const detailPending = detailId !== selectedId;

  // O detalhe fica embaixo do quadro: ao escolher uma tarefa, leva a tela até ele.
  useEffect(() => {
    if (selectedId === null) return;
    const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    detailRef.current?.scrollIntoView({ block: "nearest", behavior: calm ? "auto" : "smooth" });
  }, [selectedId]);

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

  // Com a janela em foco, o que acabou de chegar vira um aviso ali mesmo; sem foco, notificação.
  const signals = useAttentionSignals(
    attention,
    !loading && deliveriesReady,
    workspaceId,
    useCallback(
      (arrived: Attention[]) => {
        const text = notificationFor(arrived);
        if (text) say({ text: `${text.title}: ${text.body}` });
      },
      [say],
    ),
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
    // Clicar na tarefa que já está aberta não troca nada: perguntar "Descartar?" seria um aviso sem efeito.
    if (id !== null && id === selectedRef.current) return;
    const open = () => {
      dirtyRef.current = false;
      setUseChain(true);
      setSelectedId(id);
    };
    if (!dirtyRef.current) return open();
    setConfirmation({
      title: "Descartar alterações?",
      body: "Esta tarefa tem alterações não salvas. Se continuar, elas se perdem.",
      confirmLabel: "Descartar",
      danger: true,
      onConfirm: open,
    });
  }, []);
  const [dirty, setDirtyState] = useState(false);
  const setDirty = useCallback((value: boolean) => {
    dirtyRef.current = value;
    setDirtyState(value);
  }, []);

  const edit = useCallback(
    (id: number) => {
      select(id);
      // O painel monta no próximo quadro; o cursor vai para o título.
      requestAnimationFrame(() => document.getElementById("task-title")?.focus());
    },
    [select],
  );

  const remove = useCallback(
    (id: number) => {
      const task = tasksRef.current.find((item) => item.id === id);
      if (!task || workspaceId === null) return;
      setConfirmation({
        title: `Excluir a tarefa #${task.id}?`,
        body: `"${task.title}" some do Harnessy, com o histórico e as runs dela. As branches no git ficam. Não dá para desfazer.`,
        confirmLabel: "Excluir",
        danger: true,
        onConfirm: () =>
          deleteTask(workspaceId, id)
            .then(() => {
              setTasks((current) => current.filter((item) => item.id !== id));
              runsState.forgetTask(id);
              if (selectedRef.current === id) {
                dirtyRef.current = false;
                setSelectedId(null);
              }
              say({ text: `Tarefa #${id} excluída.` });
            })
            .catch((cause) => setError(message(cause))),
      });
    },
    [workspaceId, runsState.forgetTask, say],
  );

  // Mudanças de tarefa vindas do núcleo valem mesmo com o painel escondido.
  useEffect(() => {
    const stops = [
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
    setDeliveriesReady(false);
    setFilter("all");
    setTasks([]);
    setSelectedId(null);
  }, [workspaceId, workspacesReady]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `refreshKey` é o gatilho de recarga vindo do chat.
  useEffect(() => {
    if (workspaceId === null) return;
    let alive = true;
    listTasks(workspaceId)
      .then((rows) => alive && setTasks(rows))
      .catch((cause) => alive && setError(message(cause)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [workspaceId, refreshKey]);

  // Os PRs vêm do banco local e são relidos quando as runs mudam.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `runs` e `refreshKey` são gatilhos de recarga.
  useEffect(() => {
    if (workspaceId === null) return;
    let alive = true;
    listDeliveries(workspaceId)
      .then((rows) => {
        if (!alive) return;
        setDeliveries(rows);
        setDeliveriesReady(true);
      })
      .catch(() => alive && setDeliveriesReady(true));
    return () => {
      alive = false;
    };
  }, [workspaceId, runs, refreshKey]);

  // Checks, conflitos e merge acontecem no GitHub: relê os PRs abertos ao voltar para a janela e a cada 90 s.
  useEffect(() => {
    if (hidden || workspaceId === null) return;
    let busy = false;
    const sync = async () => {
      if (busy || document.visibilityState !== "visible") return;
      const open = deliveriesRef.current.filter(needsRefresh);
      if (open.length === 0) return;
      busy = true;
      // Sem `gh` ou sem rede o selo só fica como estava; o erro aparece nas ações manuais.
      await Promise.allSettled(open.map((item) => refreshDelivery(item.runId)));
      const rows = await listDeliveries(workspaceId).catch(() => null);
      if (rows) setDeliveries(rows);
      busy = false;
    };
    const timer = window.setInterval(sync, PR_POLL_MS);
    window.addEventListener("focus", sync);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", sync);
    };
  }, [hidden, workspaceId]);

  // Quem observa o quadro por leitor de tela ouve quando um estágio muda, sem perder o foco.
  useEffect(() => {
    const before = stagesRef.current;
    for (const [taskId, stage] of stages) {
      const was = before.get(taskId);
      if (was && was.id !== stage.id) {
        setLive(`Tarefa #${taskId}: ${stage.label}. ${stage.hint}`);
        break;
      }
    }
    stagesRef.current = stages;
  }, [stages]);

  // `n` leva ao campo de nova tarefa, como em qualquer lista com atalhos.
  useEffect(() => {
    if (hidden) return;
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const typing = ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
      if (typing || event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key === "n") {
        event.preventDefault();
        document.getElementById("new-task-title")?.focus();
      }
      // `a` executa o próximo passo da tarefa selecionada (o botão principal).
      if (event.key === "a" && runNextRef.current) {
        event.preventDefault();
        runNextRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [hidden]);

  const create = useCallback(
    async (draft: NewTaskDraft): Promise<boolean> => {
      if (workspaceId === null) return false;
      try {
        const task = await saveTask({
          workspaceId,
          id: null,
          revision: null,
          ...draft,
        });
        setTasks((current) => [task, ...current]);
        flash(task.id);
        setLive(`Tarefa "${task.title}" criada em A fazer.`);
        return true;
      } catch (cause) {
        setError(message(cause));
        return false;
      }
    },
    [workspaceId, flash],
  );

  // Iniciar uma run mantém o usuário no quadro: o painel da tarefa mostra o que o agente faz.
  async function start(taskId: number, action?: string, continueFrom: number | null = null) {
    setError("");
    return runsState.start(taskId, choice, action, continueFrom);
  }

  async function resolve(runId: number) {
    try {
      await resolveBlock(runId);
      say({ text: "Aviso dispensado. O estado da tarefa não mudou." });
    } catch (cause) {
      setError(message(cause));
    }
  }

  function quick(id: QuickId) {
    if (!selected) return;
    const last = runs.find((run) => run.taskId === selected.id) ?? null;
    switch (id) {
      case "refine":
        void start(selected.id, "refinar").then((run) => run && onOpenRun());
        break;
      case "implement": {
        const base = chainBase(selected, runs);
        void start(selected.id, undefined, base && useChain ? base.id : null);
        break;
      }
      case "review":
        if (last) void review(last.id);
        break;
      case "preview": {
        const work = workRun(runs, selected.id);
        if (!work) break;
        if (isLive(previews.infos[work.id])) void stopPreview(work.id).catch(fail);
        else void runPreview(work.id);
        break;
      }
      case "publish": {
        const work = workRun(runs, selected.id);
        if (work) void publish(work.id);
        break;
      }
      case "merge": {
        const work = workRun(runs, selected.id);
        if (!work) break;
        const pr = deliveryOf(deliveries, work);
        setConfirmation({
          title: `Fazer merge do PR #${pr?.prNumber ?? ""}?`,
          body: `"${selected.title}" será mesclada na branch base com squash e a tarefa vai para Concluído. O Harnessy só mescla o commit que você está vendo agora.`,
          confirmLabel: "Fazer merge",
          onConfirm: () => void merge(work.id),
        });
        break;
      }
      case "conversation":
        if (last) runsState.open(last.id);
        onOpenRun();
        break;
      case "branch":
        if (last?.branch) {
          navigator.clipboard
            .writeText(last.branch)
            .then(() => say({ text: `Branch ${last.branch} copiada.` }))
            .catch(() =>
              setError("Não foi possível copiar. Selecione o nome da branch na conversa."),
            );
        }
        break;
      case "finish":
        move(selected.id, "done");
        break;
      case "reopen":
        move(selected.id, "todo");
        break;
    }
  }

  const fail = (cause: unknown) => setError(message(cause));

  /** Rodar localmente: pede o comando se faltar, confirma o texto do repositório, e sobe. */
  async function runPreview(runId: number) {
    if (workspaceId === null) return;
    setError("");
    try {
      const profile = await getProfile(workspaceId);
      if (!profile.exists) {
        setSetup({ runId, suggestion: profile.suggestion ?? "", error: profile.error });
        return;
      }
      if (!profile.approved) {
        setConfirmation({
          title: `Rodar “${profile.run}”?`,
          body: "Este comando está em .harnessy/workspace.toml e vai rodar neste computador, na pasta da run. Confirme só se você o reconhece.",
          confirmLabel: "Confirmar e rodar",
          onConfirm: () =>
            approveProfile(workspaceId)
              .then(() => startPreview(runId))
              .then(() => previews.load(runId))
              .catch(fail),
        });
        return;
      }
      await startPreview(runId);
      await previews.load(runId);
    } catch (cause) {
      fail(cause);
    }
  }

  /** Reabre o diálogo com o comando atual, para trocá-lo (por exemplo, para usar `{port}`). */
  async function editCommand(runId: number) {
    if (workspaceId === null) return;
    try {
      const profile = await getProfile(workspaceId);
      setSetup({ runId, suggestion: profile.run ?? profile.suggestion ?? "", error: null });
    } catch (cause) {
      fail(cause);
    }
  }

  async function saveSetup(run: string) {
    if (workspaceId === null || !setup) return;
    const { runId } = setup;
    await saveProfile(workspaceId, run, null);
    setSetup(null);
    await startPreview(runId).catch(fail);
    await previews.load(runId).catch(fail);
  }

  /** Executa uma ação de entrega com feedback; o erro vem do núcleo, já em português. */
  async function deliver(action: () => Promise<Delivery>, done: (result: Delivery) => string) {
    setError("");
    setDelivering(true);
    try {
      const result = await action();
      setDeliveries((current) => [
        ...current.filter((item) => item.runId !== result.runId),
        result,
      ]);
      say({ text: done(result) });
    } catch (cause) {
      setError(message(cause));
    } finally {
      setDelivering(false);
    }
  }

  const publish = (runId: number) =>
    deliver(
      () =>
        deliveries.some((item) => item.runId === runId)
          ? refreshDelivery(runId)
          : publishRun(runId),
      (result) => `PR #${result.prNumber} ${result.draft ? "em draft" : "aberto"}.`,
    );
  const merge = (runId: number) =>
    deliver(
      () => mergeDelivery(runId),
      (result) => `PR #${result.prNumber} mergeado.`,
    );

  async function review(runId: number) {
    setError("");
    try {
      await reviewRun(runId, choice);
      runsState.open(runId);
      say({ text: "Revisão pedida. Os achados aparecem nas alterações e você será avisado." });
    } catch (cause) {
      setError(message(cause));
    }
  }

  const workspace = workspaces.find((item) => item.id === workspaceId) ?? null;
  const startable = canStartRun(agentReady, runs);
  const activeRun = selected
    ? (runs.find((run) => run.taskId === selected.id && isActive(run)) ?? null)
    : null;
  const [nowMs, setNowMs] = useState(() => Date.now());
  const activeId = activeRun?.id;
  useEffect(() => {
    if (activeId === undefined) return;
    setNowMs(Date.now());
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [activeId]);
  // O log da run em andamento precisa estar carregado e acompanhado para o painel mostrar a atividade.
  const { openRun, open: openRunLog } = runsState;
  useEffect(() => {
    if (!hidden && activeId !== undefined && openRun !== activeId) openRunLog(activeId);
  }, [hidden, activeId, openRun, openRunLog]);
  const progress = activeRun
    ? progressOf(activeRun, runsState.eventsByRun[activeRun.id], nowMs)
    : null;
  const selectedWork = selected ? workRun(runs, selected.id) : null;
  const quickInput = selected
    ? {
        task: selected,
        lastRun: runs.find((run) => run.taskId === selected.id) ?? null,
        canStart: startable && !delivering,
        starting,
        dirty,
        workRun: selectedWork,
        delivery: deliveryOf(deliveries, selectedWork),
        preview: previews.infos[selectedWork?.id ?? -1] ?? null,
      }
    : null;
  // O ambiente já ter sido ligado conta como "testei": o próximo passo deixa de ser testar.
  const next = quickInput
    ? nextAction({
        ...quickInput,
        previewSeen: selectedWork ? previews.infos[selectedWork.id] !== undefined : false,
      })
    : null;
  const workId = selectedWork?.id;
  const { ensureEvents } = runsState;
  useEffect(() => {
    if (workId !== undefined) ensureEvents(workId);
  }, [workId, ensureEvents]);
  runNextRef.current = next ? () => quick(next.id) : null;

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
                const next = Number(event.target.value);
                const change = () => {
                  dirtyRef.current = false;
                  onWorkspaceChange(next);
                };
                if (!dirtyRef.current) return change();
                setConfirmation({
                  title: "Descartar alterações?",
                  body: "Esta tarefa tem alterações não salvas. Se trocar de workspace, elas se perdem.",
                  confirmLabel: "Descartar",
                  danger: true,
                  onConfirm: change,
                });
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
            {!loading && tasks.length > 0 && (
              <div className="attention-bar">
                <fieldset className="attention-filters">
                  <legend className="sr-only">Filtrar tarefas</legend>
                  {(
                    [
                      ["all", "Todas", tasks.length],
                      ["me", "Precisa de você", attention.size],
                      ["agents", "Com os agentes", withAgentsCount],
                    ] as const
                  ).map(([id, label, count]) => (
                    <button
                      key={id}
                      type="button"
                      className={`attention-filter${id === "me" && count > 0 ? " is-urgent" : ""}`}
                      aria-pressed={filter === id}
                      onClick={() => setFilter(id)}
                    >
                      {label} <span className="attention-count">{count}</span>
                    </button>
                  ))}
                </fieldset>
                <p className="attention-phrase" role="status">
                  {countPhrase(attention.size)}
                </p>
              </div>
            )}
            {signals.digest.length > 0 && (
              <section className="digest" aria-labelledby="digest-title">
                <h3 id="digest-title">Enquanto você esteve fora</h3>
                <ul>
                  {byUrgency(signals.digest)
                    .slice(0, 5)
                    .map((item) => (
                      <li key={item.taskId}>
                        <button
                          type="button"
                          className="link-button"
                          onClick={() => {
                            setFilter("me");
                            select(item.taskId);
                            signals.dismissDigest();
                          }}
                        >
                          #{item.taskId} {item.title}
                        </button>{" "}
                        · {item.reason}
                      </li>
                    ))}
                </ul>
                {signals.digest.length > 5 && (
                  <p className="muted">e mais {signals.digest.length - 5}</p>
                )}
                <div className="digest-actions">
                  <button
                    type="button"
                    className="button primary"
                    onClick={() => {
                      setFilter("me");
                      signals.dismissDigest();
                    }}
                  >
                    Ver o que precisa de mim
                  </button>
                  <button type="button" className="button" onClick={signals.dismissDigest}>
                    Dispensar
                  </button>
                </div>
              </section>
            )}
            {loading ? (
              <KanbanSkeleton />
            ) : (
              <Kanban
                groups={groups}
                selectedId={selectedId}
                working={working}
                blocked={blocked}
                stages={stages}
                testing={testing}
                flashId={flashId}
                onSelect={select}
                onMove={move}
                onEdit={edit}
                onDelete={remove}
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
          </div>
          {selected && quickInput && (
            <QuickActions
              taskId={selected.id}
              title={selected.title}
              states={quickStates(quickInput)}
              next={next}
              working={working.has(selected.id)}
              progress={progress}
              onCancel={() => activeRun && void cancelRun(activeRun.id).catch(fail)}
              onOpenChat={() => {
                if (activeRun) runsState.open(activeRun.id);
                onOpenRun();
              }}
              onRun={quick}
              onDelete={() => remove(selected.id)}
            />
          )}
          {selected && (
            <section
              ref={detailRef}
              className="detail-panel"
              aria-label={`Detalhes da tarefa #${selected.id}`}
            >
              {selectedWork && (
                <ChangesPanel
                  key={selectedWork.id}
                  runId={selectedWork.id}
                  taskId={selected.id}
                  findings={reviewFindings(runsState.eventsByRun[selectedWork.id])}
                  defaultOpen={selected.status === "review"}
                />
              )}
              {(() => {
                const work = workRun(runs, selected.id);
                const info = work ? previews.infos[work.id] : undefined;
                return work && info ? (
                  <PreviewPanel
                    taskId={selected.id}
                    info={info}
                    log={previews.logs[work.id] ?? []}
                    onOpen={() => void openPreview(work.id).catch(fail)}
                    onStop={() => void stopPreview(work.id).catch(fail)}
                    onRestart={() => void runPreview(work.id)}
                    onEditCommand={() => void editCommand(work.id)}
                  />
                ) : null;
              })()}
              {detailPending ? (
                <TaskDetailSkeleton />
              ) : (
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
                  implementIsNext={!selectedWork || next?.id === "implement"}
                  onDirty={setDirty}
                  onClose={() => select(null)}
                  useChain={useChain}
                  onUseChain={setUseChain}
                />
              )}
              {!startable && agentReady && (
                <p className="muted">Já existe uma run ativa neste workspace.</p>
              )}
            </section>
          )}
        </div>
      )}
      <PreviewSetup open={setup} onClose={() => setSetup(null)} onSave={saveSetup} />
      <ConfirmDialog ask={confirmation} onClose={() => setConfirmation(null)} />
    </section>
  );
}
