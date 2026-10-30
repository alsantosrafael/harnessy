import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  ChevronRight,
  CircleHelp,
  Columns3,
  Command,
  ExternalLink,
  FolderGit2,
  Layers3,
  MessageSquare,
  PanelRightClose,
  PanelRightOpen,
  Plug,
  Plus,
  ShieldCheck,
  X,
} from "lucide-react";
import { type AgentChoice, loadChoice, saveChoice } from "./lib/agents";
import { getAppInfo, listWorkspaces, type AppInfo, type Workspace } from "./lib/desktop";
import { createDraftSession } from "./lib/drafts";
import { BoardPanel } from "./components/BoardPanel";
import { ChatPanel } from "./components/ChatPanel";
import { ConnectionsPage } from "./components/ConnectionsPage";
import { ContextPanel } from "./components/ContextPanel";

type Page = "workspace" | "connections";
type Tab = "board" | "chat";
const tabNames = ["board", "chat"] as const;

export function App() {
  const [page, setPage] = useState<Page>("workspace");
  const [tab, setTab] = useState<Tab>("board");
  const [details, setDetails] = useState(true);
  // The session belongs to the application, not to a screen that can unmount.
  const [draftSession] = useState(createDraftSession);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [runtimeError, setRuntimeError] = useState(false);
  // Workspaces, agente e modelo valem para o chat e para as tarefas.
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspacesReady, setWorkspacesReady] = useState(false);
  const [workspaceId, setWorkspaceId] = useState<number | null>(null);
  const [loadError, setLoadError] = useState("");
  const [boardTick, setBoardTick] = useState(0);
  const [choice, setChoice] = useState<AgentChoice>(loadChoice);
  const [help, setHelp] = useState(false);
  const helpButton = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);

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
  }

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
      if ((event.metaKey || event.ctrlKey) && ["1", "2"].includes(event.key)) {
        event.preventDefault();
        setPage(event.key === "1" ? "workspace" : "connections");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (help) dialog.current?.showModal();
    else if (dialog.current?.open) dialog.current.close();
  }, [help]);

  function switchTab(next: Tab) {
    setPage("workspace");
    setTab(next);
    if (next === "chat") requestAnimationFrame(() => document.getElementById("prompt")?.focus());
  }

  const connect = () => setPage("connections");
  const runtimeLabel = runtimeError
    ? "Núcleo indisponível"
    : info
      ? "Núcleo local conectado"
      : "Prévia da interface";

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
          <span className="local-label">LOCAL</span>
        </div>
        <div className="workspace-label">
          <span className="workspace-avatar" aria-hidden="true">
            P
          </span>
          <div>
            Meu espaço<span>Pessoal · neste dispositivo</span>
          </div>
        </div>
        <p className="section-label">WORKSPACE</p>
        <nav>
          <button
            className={`nav-item ${page === "workspace" ? "selected" : ""}`}
            aria-current={page === "workspace" ? "page" : undefined}
            onClick={() => setPage("workspace")}
          >
            <Layers3 size={16} aria-hidden="true" />
            Visão geral<span className="shortcut">⌘1</span>
          </button>
          <button
            className={`nav-item ${page === "connections" ? "selected" : ""}`}
            aria-current={page === "connections" ? "page" : undefined}
            onClick={connect}
          >
            <Plug size={16} aria-hidden="true" />
            Conexões<span className="shortcut">⌘2</span>
          </button>
        </nav>
        <div className="section-row">
          <p className="section-label">CONVERSAS</p>
          <button
            className="icon-button"
            aria-label="Abrir rascunho de conversa"
            onClick={() => switchTab("chat")}
          >
            <Plus size={15} aria-hidden="true" />
          </button>
        </div>
        <button
          className={`nav-item conversation ${page === "workspace" && tab === "chat" ? "active-conversation" : ""}`}
          onClick={() => switchTab("chat")}
        >
          <MessageSquare size={15} aria-hidden="true" />
          <span>Próxima tarefa</span>
          <span className="draft-label">rascunho</span>
        </button>
        <div className="sidebar-bottom">
          <div className="privacy-note">
            <ShieldCheck size={16} aria-hidden="true" />
            <div>
              Seu código. Seu controle.<span>Execução local, por intenção.</span>
            </div>
          </div>
          <button ref={helpButton} className="nav-item" onClick={() => setHelp(true)}>
            <CircleHelp size={16} aria-hidden="true" />
            Sobre esta fundação<span className="version">v{__APP_VERSION__}</span>
          </button>
        </div>
      </aside>

      <div className="main-shell">
        <header className="topbar">
          <div>
            <FolderGit2 size={15} aria-hidden="true" />
            <span>Meu espaço</span>
            <ChevronRight size={13} aria-hidden="true" />
            <strong>{page === "connections" ? "Conexões" : "Visão geral"}</strong>
          </div>
          <span className="runtime-status">
            <span className={`status-dot ${runtimeError ? "status-error" : ""}`} />
            {runtimeLabel}
          </span>
        </header>

        <main id="main" tabIndex={-1}>
          {page === "connections" && <ConnectionsPage />}
          <section className="page workspace-page" hidden={page !== "workspace"}>
            <div className="page-heading">
              <div>
                <p className="eyebrow">DA INTENÇÃO À ENTREGA</p>
                <h1>
                  Menos operação.
                  <br />
                  Mais espaço para construir.
                </h1>
                <p className="lead">
                  Organize o trabalho. Oriente seus agentes. Decida o que vai para o mundo.
                </p>
              </div>
              <button className="button primary" onClick={() => switchTab("chat")}>
                <Plus size={16} aria-hidden="true" />
                Preparar tarefa
              </button>
            </div>
            <div className="setup-banner">
              <span className="setup-icon" aria-hidden="true">
                <Plug size={19} />
              </span>
              <div>
                <strong>Seu projeto começa aqui.</strong>
                <p>
                  Escreva a tarefa, mande o Claude Code implementar e acompanhe o card andar de A
                  fazer a Concluído. As sessões do GitHub, Codex e Claude Code ficam em Conexões.
                </p>
              </div>
              <button className="button subtle" onClick={connect}>
                Ver conexões
                <ArrowRight size={15} aria-hidden="true" />
              </button>
            </div>
            <div className="workspace-toolbar">
              <div className="tabs" role="tablist" aria-label="Área de trabalho">
                {tabNames.map((value, index) => (
                  <button
                    key={value}
                    ref={(node) => {
                      tabs.current[index] = node;
                    }}
                    id={`${value}-tab`}
                    role="tab"
                    aria-controls={`${value}-panel`}
                    aria-selected={tab === value}
                    tabIndex={tab === value ? 0 : -1}
                    onClick={() => setTab(value)}
                    onKeyDown={(event) => {
                      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
                      event.preventDefault();
                      const next =
                        event.key === "Home"
                          ? 0
                          : event.key === "End"
                            ? tabNames.length - 1
                            : (index + (event.key === "ArrowRight" ? 1 : -1) + tabNames.length) %
                              tabNames.length;
                      setTab(tabNames[next]);
                      tabs.current[next]?.focus();
                    }}
                  >
                    {value === "board" ? (
                      <Columns3 size={16} aria-hidden="true" />
                    ) : (
                      <MessageSquare size={16} aria-hidden="true" />
                    )}
                    {value === "board" ? "Tarefas" : "Chat"}
                  </button>
                ))}
              </div>
              <button
                className="icon-button"
                aria-label={details ? "Ocultar contexto" : "Mostrar contexto"}
                aria-expanded={details}
                aria-controls={details ? "context-panel" : undefined}
                onClick={() => setDetails(!details)}
              >
                {details ? (
                  <PanelRightClose size={18} aria-hidden="true" />
                ) : (
                  <PanelRightOpen size={18} aria-hidden="true" />
                )}
              </button>
            </div>
            <div className="work-area">
              <div className="work-content">
                <BoardPanel
                  hidden={tab !== "board"}
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
                />
                <ChatPanel
                  hidden={tab !== "chat"}
                  session={draftSession}
                  workspaces={workspaces}
                  workspaceId={workspaceId}
                  onWorkspaceChange={setWorkspaceId}
                  choice={choice}
                  onChoice={chooseAgent}
                  agentReady={info?.agentExecutionAvailable ?? false}
                  onCreated={(started) => {
                    setBoardTick((tick) => tick + 1);
                    if (started) switchTab("board");
                  }}
                />
              </div>
              {details && <ContextPanel />}
            </div>
          </section>
        </main>

        <footer className="statusbar">
          <span>
            <span className={`status-dot ${runtimeError ? "status-error" : ""}`} />
            {runtimeError
              ? "Erro de comunicação com o núcleo"
              : "Trabalho local · conexões verificadas pelas CLIs · agentes executam só por ação sua"}
          </span>
          <span>
            <Command size={11} aria-hidden="true" />1 Workspace
            <span className="status-separator" />
            ⌘2 Conexões
          </span>
        </footer>
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
        <span className="brand-mark" aria-hidden="true">
          h
        </span>
        <h2 id="about-title">Uma fundação para construir.</h2>
        <p>
          Harnessy {info?.version ?? __APP_VERSION__} · {info?.platform ?? "prévia web"}
        </p>
        <p>
          Interface React, núcleo Rust e janela Tauri. Organize workspaces e tarefas em SQLite e
          prepare um rascunho local. GitHub, Codex, execução, deploy e sandbox ainda não estão
          implementados.
        </p>
        <p>
          Fechar a janela mantém o aplicativo ativo. Use o menu “H” na barra do macOS para reabrir
          ou encerrar.
        </p>
        <span className="badge">
          <ExternalLink size={12} aria-hidden="true" />
          Local-first · Sem agentes ativos
        </span>
      </dialog>
    </div>
  );
}
