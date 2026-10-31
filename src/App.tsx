import { useCallback, useEffect, useRef, useState } from "react";
import {
  ChevronRight,
  CircleHelp,
  Columns3,
  Folder,
  MessageSquare,
  Plug,
  SquarePen,
  X,
} from "lucide-react";
import {
  type AgentChoice,
  hasStoredChoice,
  loadChoice,
  normalizeChoice,
  PREFERENCE_KEY,
  saveChoice,
} from "./lib/agents";
import {
  getAppInfo,
  getPreference,
  listWorkspaces,
  setPreference,
  type AppInfo,
  type Workspace,
} from "./lib/desktop";
import { createDraftSession } from "./lib/drafts";
import { isActive } from "./lib/runs";
import { useRuns } from "./lib/useRuns";
import { BoardPanel } from "./components/BoardPanel";
import { ChatPanel } from "./components/ChatPanel";
import { ConnectionsPage } from "./components/ConnectionsPage";

type View = "board" | "chat" | "connections";
const shortcuts: Record<string, View> = { "1": "board", "2": "chat", "3": "connections" };

export function App() {
  const [view, setView] = useState<View>("board");
  // The session belongs to the application, not to a screen that can unmount.
  const [draftSession] = useState(createDraftSession);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [runtimeError, setRuntimeError] = useState(false);
  // Workspaces, agente e modelo valem para o chat e para as tarefas.
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspacesReady, setWorkspacesReady] = useState(false);
  const [workspaceId, setWorkspaceId] = useState<number | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  // O workspace que passa a ser o atual começa aberto; depois disso o usuário pode recolhê-lo.
  useEffect(() => {
    if (workspaceId === null) return;
    setExpanded((set) => (set.has(workspaceId) ? set : new Set(set).add(workspaceId)));
  }, [workspaceId]);
  const [loadError, setLoadError] = useState("");
  const [boardTick, setBoardTick] = useState(0);
  const [runError, setRunError] = useState("");
  const runs = useRuns(workspaceId, boardTick, setRunError);
  const [choice, setChoice] = useState<AgentChoice>(loadChoice);
  const [help, setHelp] = useState(false);
  const helpButton = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);

  const show = useCallback((next: View) => {
    setView(next);
    if (next === "chat") requestAnimationFrame(() => document.getElementById("prompt")?.focus());
  }, []);

  useEffect(() => {
    const flush = () => draftSession.flush();
    window.addEventListener("pagehide", flush);
    return () => {
      window.removeEventListener("pagehide", flush);
      flush();
    };
  }, [draftSession]);

  useEffect(() => {
    listWorkspaces()
      .then((rows) => {
        setWorkspaces(rows);
        setWorkspaceId((current) => current ?? rows[0]?.id ?? null);
      })
      .catch((cause) => setLoadError(cause instanceof Error ? cause.message : String(cause)))
      .finally(() => setWorkspacesReady(true));
  }, []);

  function chooseAgent(next: AgentChoice) {
    setChoice(next);
    saveChoice(next);
    // No banco local a escolha vale para qualquer janela (instalada ou em desenvolvimento).
    setPreference(PREFERENCE_KEY, JSON.stringify(next)).catch(() => undefined);
  }

  // A escolha do banco vence a do navegador; sem ela, a do navegador (se for explícita) migra para o banco.
  useEffect(() => {
    getPreference(PREFERENCE_KEY)
      .then((raw) => {
        if (raw) {
          setChoice(normalizeChoice(JSON.parse(raw)));
        } else if (hasStoredChoice()) {
          setPreference(PREFERENCE_KEY, JSON.stringify(loadChoice())).catch(() => undefined);
        }
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    let alive = true;
    getAppInfo()
      .then((result) => {
        if (alive) setInfo(result);
      })
      .catch(() => {
        if (alive) setRuntimeError(true);
      });
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (dialog.current?.open) return;
      const next = shortcuts[event.key];
      if ((event.metaKey || event.ctrlKey) && next) {
        event.preventDefault();
        show(next);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [show]);

  useEffect(() => {
    if (help) dialog.current?.showModal();
    else if (dialog.current?.open) dialog.current.close();
  }, [help]);

  const connect = () => setView("connections");
  // Sai da run aberta: o chat volta ao estado de criar uma tarefa nova.
  const newTask = (id?: number) => {
    if (id !== undefined) setWorkspaceId(id);
    runs.open(null);
    show("chat");
  };
  function toggleWorkspace(id: number) {
    // Clicar em outro workspace o torna ativo e o abre; clicar no atual abre ou recolhe o grupo.
    if (id !== workspaceId) {
      setWorkspaceId(id);
      setExpanded((set) => new Set(set).add(id));
      // Outro workspace abre nas tarefas dele, como a navegação por workspace promete.
      show("board");
      return;
    }
    setExpanded((set) => {
      const next = new Set(set);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }
  const working = runs.runs.filter(isActive).length;
  const runtimeLabel = runtimeError ? "Núcleo indisponível" : info ? "Local" : "Prévia";
  const workspace = workspaces.find((item) => item.id === workspaceId);
  const viewTitle = { board: "Tarefas", chat: "Chat", connections: "Conexões" }[view];

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Pular para o conteúdo
      </a>
      <aside className="sidebar" aria-label="Navegação principal">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            h
          </span>
          <strong>harnessy</strong>
        </div>
        <button className="button primary new-task" onClick={() => newTask()}>
          <SquarePen size={14} aria-hidden="true" />
          Nova tarefa
        </button>
        <nav className="workspace-tree" aria-label="Workspaces">
          {workspaces.length === 0 && (
            <>
              <button
                className={`nav-item ${view === "board" ? "selected" : ""}`}
                aria-current={view === "board" ? "page" : undefined}
                onClick={() => show("board")}
              >
                <Columns3 size={16} aria-hidden="true" />
                Tarefas<span className="shortcut">⌘1</span>
              </button>
              <button
                className={`nav-item ${view === "connections" ? "selected" : ""}`}
                aria-current={view === "connections" ? "page" : undefined}
                onClick={connect}
              >
                <Plug size={16} aria-hidden="true" />
                Conexões<span className="shortcut">⌘3</span>
              </button>
            </>
          )}
          {workspaces.map((item) => {
            const current = item.id === workspaceId;
            const open = expanded.has(item.id);
            const go = (next: View) => {
              setWorkspaceId(item.id);
              show(next);
            };
            const here = (target: View) => current && view === target;
            return (
              <div className="ws-group" key={item.id}>
                <button
                  className={`nav-item ws-header ${current ? "current" : ""}`}
                  aria-expanded={open}
                  title={item.folder}
                  onClick={() => toggleWorkspace(item.id)}
                >
                  <ChevronRight size={14} className="ws-chevron" aria-hidden="true" />
                  <Folder size={16} aria-hidden="true" />
                  <span className="ws-name">{item.name}</span>
                  {current && working > 0 && (
                    <span className="nav-badge" title={`${working} run(s) em andamento`}>
                      {working}
                    </span>
                  )}
                </button>
                {open && (
                  <div className="ws-items">
                    <button
                      className={`nav-item ${here("board") ? "selected" : ""}`}
                      aria-current={here("board") ? "page" : undefined}
                      onClick={() => go("board")}
                    >
                      <Columns3 size={15} aria-hidden="true" />
                      Tarefas{current && <span className="shortcut">⌘1</span>}
                    </button>
                    <button
                      className={`nav-item ${here("chat") ? "selected" : ""}`}
                      aria-current={here("chat") ? "page" : undefined}
                      onClick={() => go("chat")}
                    >
                      <MessageSquare size={15} aria-hidden="true" />
                      Chat{current && <span className="shortcut">⌘2</span>}
                    </button>
                    <button
                      className={`nav-item ${here("connections") ? "selected" : ""}`}
                      aria-current={here("connections") ? "page" : undefined}
                      onClick={() => go("connections")}
                    >
                      <Plug size={15} aria-hidden="true" />
                      Conexões{current && <span className="shortcut">⌘3</span>}
                    </button>
                    <button className="nav-item" onClick={() => newTask(item.id)}>
                      <SquarePen size={15} aria-hidden="true" />
                      Nova tarefa
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </nav>
        <div className="sidebar-bottom">
          <button ref={helpButton} className="nav-item" onClick={() => setHelp(true)}>
            <CircleHelp size={16} aria-hidden="true" />
            Sobre
            <span className="version">
              v{__APP_VERSION__}
              {info?.instance ? ` · ${info.instance}` : ""}
            </span>
          </button>
        </div>
      </aside>

      <div className="main-shell">
        <header className="topbar">
          <nav className="breadcrumb" aria-label="Localização">
            {workspace && view !== "connections" && <span>{workspace.name}</span>}
            {workspace && view !== "connections" && <span aria-hidden="true">/</span>}
            <strong>{viewTitle}</strong>
          </nav>
          {info?.instance && (
            <span
              className="instance-pill"
              title="Instância de teste: banco, preferências, branches e ícone próprios, separados do seu Harnessy principal."
            >
              Instância de teste · {info.instance}
            </span>
          )}
          <span
            className="runtime-status"
            title={runtimeError ? "Erro de comunicação com o núcleo" : undefined}
          >
            <span className={`status-dot ${runtimeError ? "status-error" : ""}`} />
            {runtimeLabel}
          </span>
        </header>

        {runError && (
          <p className="local-error app-error" role="alert">
            {runError}{" "}
            <button type="button" className="link-button" onClick={() => setRunError("")}>
              Dispensar
            </button>
          </p>
        )}
        <main id="main" tabIndex={-1}>
          {view === "connections" && <ConnectionsPage />}
          <div className="page workspace-page" hidden={view === "connections"}>
            <BoardPanel
              hidden={view !== "board"}
              agentReady={info?.agentExecutionAvailable ?? false}
              onConnect={connect}
              workspaces={workspaces}
              workspacesReady={workspacesReady}
              workspaceId={workspaceId}
              loadError={loadError}
              onWorkspaceChange={setWorkspaceId}
              onWorkspaceCreated={(created) => {
                setWorkspaces((rows) => [...rows, created]);
                setWorkspaceId((current) => current ?? created.id);
              }}
              refreshKey={boardTick}
              choice={choice}
              onChoice={chooseAgent}
              runs={runs}
              onOpenRun={() => show("chat")}
            />
            <ChatPanel
              hidden={view !== "chat"}
              session={draftSession}
              workspaces={workspaces}
              workspaceId={workspaceId}
              onWorkspaceChange={setWorkspaceId}
              choice={choice}
              onChoice={chooseAgent}
              agentReady={info?.agentExecutionAvailable ?? false}
              runs={runs}
              onCreated={() => setBoardTick((tick) => tick + 1)}
              onStepsCreated={() => setBoardTick((tick) => tick + 1)}
              onTaskChanged={() => setBoardTick((tick) => tick + 1)}
              onError={setRunError}
            />
          </div>
        </main>
      </div>

      <dialog
        ref={dialog}
        className="about-dialog"
        aria-labelledby="about-title"
        onCancel={() => setHelp(false)}
        onClose={() => {
          setHelp(false);
          helpButton.current?.focus();
        }}
      >
        <button
          // biome-ignore lint/a11y/noAutofocus: focus starts inside the opened modal and returns to its trigger.
          autoFocus
          className="icon-button dialog-close"
          aria-label="Fechar"
          onClick={() => setHelp(false)}
        >
          <X size={18} aria-hidden="true" />
        </button>
        <h2 id="about-title">Harnessy</h2>
        <p>
          Versão {info?.version ?? __APP_VERSION__} · {info?.platform ?? "prévia web"}
        </p>
        <p>
          Fechar a janela mantém o aplicativo ativo. Use o menu “H” na barra do macOS para reabrir
          ou encerrar.
        </p>
      </dialog>
    </div>
  );
}
