import { useRef, useState, useSyncExternalStore } from "react";
import { ArrowDown, Send, Sparkles } from "lucide-react";
import type { AgentChoice } from "../lib/agents";
import { splitDraft } from "../lib/chatTask";
import { saveTask, startRun, type Workspace } from "../lib/desktop";
import { DRAFT_LIMIT, type DraftSession } from "../lib/drafts";
import { AgentPicker } from "./AgentPicker";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

const suggestions = [
  "Implementar uma tarefa",
  "Revisar um pull request",
  "Planejar um novo projeto",
];

interface Props {
  hidden: boolean;
  session: DraftSession;
  workspaces: Workspace[];
  workspaceId: number | null;
  onWorkspaceChange: (id: number) => void;
  choice: AgentChoice;
  onChoice: (choice: AgentChoice) => void;
  /** Claude Code e git encontrados neste computador. */
  agentReady: boolean;
  /** Tarefa criada; `started` indica que a run também começou. */
  onCreated: (started: boolean) => void;
}

export function ChatPanel({
  hidden,
  session,
  workspaces,
  workspaceId,
  onWorkspaceChange,
  choice,
  onChoice,
  agentReady,
  onCreated,
}: Props) {
  const { text: draft, status: saveState } = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
  );
  const input = useRef<HTMLTextAreaElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  // Dito em texto, não só no botão desabilitado: o que falta para iniciar.
  const blockedBy = !agentReady
    ? "Claude Code ou git não foram encontrados. Veja em Conexões."
    : workspaceId === null
      ? "Crie um workspace em Tarefas para guardar a tarefa."
      : !draft.trim()
        ? "Escreva a tarefa para poder iniciar."
        : "";

  async function start() {
    if (blockedBy || busy || workspaceId === null) return;
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
    try {
      await startRun(workspaceId, taskId, choice.runtime, choice.model);
      onCreated(true);
    } catch (cause) {
      setError(`A tarefa foi criada em A fazer, mas a run não começou: ${message(cause)}`);
      onCreated(false);
    } finally {
      setBusy(false);
    }
  }

  function insertSuggestion(value: string) {
    // Suggestions never destroy existing work; focus the composer after insertion.
    session.appendSuggestion(value);
    input.current?.focus();
  }

  const status =
    saveState === "saving"
      ? "Salvando rascunho…"
      : saveState === "error"
        ? "Não foi possível salvar. Mantenha esta janela aberta para preservar o texto."
        : saveState === "blocked"
          ? "O registro anterior não pôde ser lido. Ele foi preservado; seu novo texto permanece apenas na memória."
          : saveState === "empty"
            ? "Seu rascunho será salvo neste dispositivo quando você começar a escrever."
            : "Rascunho salvo neste dispositivo.";

  return (
    <section id="chat-panel" role="tabpanel" aria-labelledby="chat-tab" hidden={hidden}>
      <div className="chat-intro">
        <span className="chat-spark" aria-hidden="true">
          <Sparkles size={24} strokeWidth={1.4} />
        </span>
        <h2>O que vamos construir?</h2>
        <p>
          Descreva o que quer fazer. Ao iniciar, o Harnessy cria a tarefa e manda o agente que você
          escolher implementar.
        </p>
      </div>
      {/* biome-ignore lint/a11y/useSemanticElements: suggestions are a group, not form fields. */}
      <div
        role="group"
        className="prompt-suggestions"
        aria-label="Sugestões para adicionar ao rascunho"
      >
        {suggestions.map((text) => (
          <button key={text} type="button" onClick={() => insertSuggestion(text)}>
            {text}
            <ArrowDown size={13} aria-hidden="true" />
          </button>
        ))}
      </div>
      <form
        className="composer"
        onSubmit={(event) => {
          event.preventDefault();
          void start();
        }}
      >
        <label htmlFor="prompt">
          Próxima tarefa <span>Rascunho local</span>
        </label>
        <textarea
          ref={input}
          id="prompt"
          maxLength={DRAFT_LIMIT}
          value={draft}
          onChange={(event) => session.edit(event.target.value)}
          placeholder="Descreva o que você quer fazer…"
          aria-describedby="start-hint draft-status"
          rows={4}
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
            disabled={Boolean(blockedBy) || busy}
            aria-describedby="start-hint"
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
      <p id="start-hint" className="composer-hint">
        {blockedBy || "Cria a tarefa em A fazer e inicia a run com o agente e o modelo escolhidos."}
      </p>
      {error && (
        <p className="local-error composer-error" role="alert">
          {error}
        </p>
      )}
      <p className="secrets-note">Não inclua senhas, tokens ou conteúdo de arquivos .env.</p>
    </section>
  );
}
