import { useRef, useSyncExternalStore } from "react";
import { ArrowDown, Send, Sparkles, Terminal } from "lucide-react";
import { DRAFT_LIMIT, type DraftSession } from "../lib/drafts";

const suggestions = [
  "Implementar uma tarefa",
  "Revisar um pull request",
  "Planejar um novo projeto",
];

export function ChatPanel({ hidden, session }: { hidden: boolean; session: DraftSession }) {
  const { text: draft, status: saveState } = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
  );
  const input = useRef<HTMLTextAreaElement>(null);

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
          Comece com uma ideia. O refinador ajuda a transformá-la em uma tarefa clara antes da
          implementação.
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
      <form className="composer" onSubmit={(event) => event.preventDefault()}>
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
          aria-describedby="send-unavailable draft-status"
          rows={4}
        />
        <div className="composer-actions">
          <span>
            <Terminal size={13} aria-hidden="true" />
            Codex não conectado
          </span>
          <button className="button primary" disabled aria-describedby="send-unavailable">
            Iniciar
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
      <p id="send-unavailable" className="composer-hint">
        O envio será habilitado com a integração Codex.
      </p>
      <p className="secrets-note">Não inclua senhas, tokens ou conteúdo de arquivos .env.</p>
    </section>
  );
}
