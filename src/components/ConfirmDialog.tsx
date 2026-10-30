import { useEffect, useRef } from "react";

export interface Confirmation {
  title: string;
  body: string;
  confirmLabel: string;
  /** Ação irreversível: o botão de confirmar usa a cor de perigo. */
  danger?: boolean;
  onConfirm: () => void;
}

interface Props {
  ask: Confirmation | null;
  onClose: () => void;
}

/**
 * Confirmação dentro do app. O `window.confirm` da webview do macOS pode devolver
 * `false` sem mostrar nada, e a ação some em silêncio.
 */
export function ConfirmDialog({ ask, onClose }: Props) {
  const box = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = box.current;
    if (!dialog) return;
    if (ask && !dialog.open) dialog.showModal();
    if (!ask && dialog.open) dialog.close();
  }, [ask]);

  return (
    <dialog
      ref={box}
      className="confirm-dialog"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-body"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      {ask && (
        <form method="dialog" onSubmit={(event) => event.preventDefault()}>
          <h2 id="confirm-title">{ask.title}</h2>
          <p id="confirm-body">{ask.body}</p>
          <div className="confirm-actions">
            {/* biome-ignore lint/a11y/noAutofocus: o foco começa no botão seguro, não no destrutivo. */}
            <button type="button" className="button" autoFocus onClick={onClose}>
              Cancelar
            </button>
            <button
              type="button"
              className={`button ${ask.danger ? "is-danger" : "primary"}`}
              onClick={() => {
                onClose();
                ask.onConfirm();
              }}
            >
              {ask.confirmLabel}
            </button>
          </div>
        </form>
      )}
    </dialog>
  );
}
