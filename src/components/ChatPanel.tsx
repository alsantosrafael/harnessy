import { Send } from "lucide-react";
import { useState, useSyncExternalStore } from "react";
import type { AgentChoice } from "../lib/agents";
import { splitDraft } from "../lib/chatTask";
import { saveTask, type LocalTask, type Workspace } from "../lib/desktop";
import { DRAFT_LIMIT, type DraftSession } from "../lib/drafts";
import type { RunsState } from "../lib/useRuns";
import { AgentPicker } from "./AgentPicker";
import { RunFeed } from "./RunFeed";
import { RunList } from "./RunList";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

interface Props {
  hidden: boolean;
  session: DraftSession;
  workspaces: Workspace[];
  workspaceId: number | null;
  onWorkspaceChange: (id: number) => void;
  choice: AgentChoice;
  onChoice: (choice: AgentChoice) => void;
  /** Agente e git encontrados neste computador. */
  agentReady: boolean;
  /** Runs compartilhadas com o quadro; a conversa mostra a aberta. */
  runs: RunsState;
  /** Tarefa criada (e, com `started`, a run começou): o quadro recarrega. */
  onCreated: (started: boolean) => void;
  /** Passos confirmados viraram tarefas. */
  onStepsCreated: (tasks: LocalTask[]) => void;
  onError: (text: string) => void;
}

/** Chat: runs à esquerda, conversa ao vivo do agente e, embaixo, o campo de nova tarefa. */
export function ChatPanel({
  hidden,
  session,
  workspaces,
  workspaceId,
  onWorkspaceChange,
  choice,
  onChoice,
  agentReady,
  runs: runsState,
  onCreated,
  onStepsCreated,
  onError,
}: Props) {
  const { text: draft, status: saveState } = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const { runs, openRun, events } = runsState;
  const shown = runs.find((run) => run.id === openRun) ?? null;

  // Dito em texto, não só no botão desabilitado: o que falta para iniciar.
  const blockedBy = !agentReady
    ? "Agente ou git não foram encontrados. Veja em Conexões."
    : workspaceId === null
      ? "Crie um workspace em Tarefas para guardar a tarefa."
      : !draft.trim()
        ? ""
        : "";

  async function start() {
    if (!agentReady || busy || workspaceId === null || !draft.trim()) return;
    setBusy(true);
    setError("");
    const { title, description } = splitDraft(draft);
    let taskId: number;
    try {
      const task = await saveTask({
        workspaceId,
        id: null,
        revision: null,
        title,
        description,
        criteria: "",
      });
      taskId = task.id;
    } catch (cause) {
      setError(message(cause));
      setBusy(false);
      return;
    }
    // A tarefa já está salva: o rascunho cumpriu o papel dele.
    session.edit("");
    session.flush();
    const run = await runsState.start(taskId, choice);
    if (!run) setError("A tarefa foi criada em A fazer, mas a run não começou.");
    onCreated(run !== null);
    setBusy(false);
  }

  const status =
    saveState === "saving"
      ? "Salvando…"
      : saveState === "error"
        ? "Não foi possível salvar. Mantenha esta janela aberta para preservar o texto."
        : saveState === "blocked"
          ? "O registro anterior não pôde ser lido. Ele foi preservado; seu novo texto permanece apenas na memória."
          : saveState === "empty"
            ? ""
            : "Rascunho salvo";

  return (
    <section id="chat-panel" className="chat-panel" aria-label="Chat" hidden={hidden}>
      <div className="chat-layout">
        <RunList runs={runs} openRun={openRun} onOpen={runsState.open} />
        <div className="chat-main">
          {shown ? (
            <RunFeed
              run={shown}
              events={events}
              onError={onError}
              onStepsCreated={onStepsCreated}
            />
          ) : (
            <div className="chat-empty">
              <h1>O que vamos construir?</h1>
              <p className="muted">
                Descreva uma tarefa abaixo. O agente trabalha e a conversa aparece aqui, ao vivo.
              </p>
            </div>
          )}
          <form
            className="composer"
            onSubmit={(event) => {
              event.preventDefault();
              void start();
            }}
          >
            <label className="sr-only" htmlFor="prompt">
              Descrição da tarefa
            </label>
            <textarea
              id="prompt"
              maxLength={DRAFT_LIMIT}
              value={draft}
              onChange={(event) => session.edit(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                  event.preventDefault();
                  void start();
                }
              }}
              placeholder="Nova tarefa: o que o agente deve fazer? Não inclua senhas, tokens ou arquivos .env."
              aria-describedby={blockedBy ? "start-hint draft-status" : "draft-status"}
              rows={3}
            />
            <div className="composer-actions">
              <div className="composer-controls">
                <label className="chat-workspace">
                  Workspace
                  <select
                    value={workspaceId ?? ""}
                    disabled={workspaces.length === 0 || busy}
                    onChange={(event) => onWorkspaceChange(Number(event.target.value))}
                  >
                    {workspaces.length === 0 && <option value="">Nenhum workspace</option>}
                    {workspaces.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.name}
                      </option>
                    ))}
                  </select>
                </label>
                <AgentPicker choice={choice} onChange={onChoice} />
              </div>
              <button
                className="button primary"
                disabled={Boolean(blockedBy) || busy || !draft.trim()}
                aria-describedby={blockedBy ? "start-hint" : undefined}
              >
                {busy ? "Iniciando…" : "Iniciar"}
                <Send size={14} aria-hidden="true" />
              </button>
            </div>
          </form>
          <p
            id="draft-status"
            className={`draft-status ${saveState === "error" || saveState === "blocked" ? "save-error" : ""}`}
            role="status"
            aria-live="polite"
          >
            {status}
          </p>
          {saveState === "error" && (
            <button className="button" onClick={session.flush}>
              Tentar salvar novamente
            </button>
          )}
          {saveState === "blocked" && (
            <div className="draft-recovery">
              <p>
                Copie o registro antigo antes de substituí-lo, caso precise recuperá-lo. Esta ação
                substitui o registro anterior pelo texto atual.
              </p>
              <button className="button" onClick={session.replaceUnreadableRecord}>
                Substituir registro anterior pelo texto atual
              </button>
            </div>
          )}
          {blockedBy && (
            <p id="start-hint" className="composer-hint">
              {blockedBy}
            </p>
          )}
          {error && (
            <p className="local-error composer-error" role="alert">
              {error}
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
