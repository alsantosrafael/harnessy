import { useEffect, useRef, useState } from "react";

interface Props {
  /** Abre com o comando sugerido; `null` mantém o diálogo fechado. */
  open: { suggestion: string; error: string | null } | null;
  onClose: () => void;
  onSave: (run: string) => Promise<void>;
}

/**
 * Define como subir o projeto. O comando roda neste computador, então o diálogo diz
 * onde ele roda e onde fica gravado.
 */
export function PreviewSetup({ open, onClose, onSave }: Props) {
  const box = useRef<HTMLDialogElement>(null);
  const [run, setRun] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState("");

  useEffect(() => {
    const dialog = box.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      setRun(open.suggestion);
      setProblem(open.error ?? "");
      dialog.showModal();
      // O diálogo existe para preencher este campo: o foco começa nele.
      requestAnimationFrame(() => document.getElementById("setup-command")?.focus());
    }
    if (!open && dialog.open) dialog.close();
  }, [open]);

  async function submit() {
    const command = run.trim();
    if (!command) return setProblem("Digite o comando que sobe o projeto.");
    setBusy(true);
    setProblem("");
    try {
      await onSave(command);
    } catch (cause) {
      setProblem(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <dialog
      ref={box}
      className="confirm-dialog setup-dialog"
      aria-labelledby="setup-title"
      aria-describedby="setup-help"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      {open && (
        <form
          method="dialog"
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <h2 id="setup-title">Como subir este projeto?</h2>
          <p id="setup-help">
            O comando roda neste computador, na pasta da run, e fica salvo em{" "}
            <code>.harnessy/workspace.toml</code> no repositório. Você confirma uma vez. Se o
            projeto usa uma porta (como `--port 3000`), escreva <code>{"{port}"}</code> no lugar do
            número: o Harnessy escolhe uma porta livre e você não precisa se preocupar com
            conflitos.
          </p>
          <label className="field">
            Comando
            <input
              id="setup-command"
              value={run}
              maxLength={300}
              placeholder="bun run dev"
              onChange={(event) => setRun(event.target.value)}
            />
          </label>
          {problem && (
            <p className="field-error" role="alert">
              {problem}
            </p>
          )}
          <div className="confirm-actions">
            <button type="button" className="button" onClick={onClose}>
              Cancelar
            </button>
            <button type="submit" className="button primary" disabled={busy}>
              {busy ? "Salvando…" : "Salvar e rodar"}
            </button>
          </div>
        </form>
      )}
    </dialog>
  );
}
