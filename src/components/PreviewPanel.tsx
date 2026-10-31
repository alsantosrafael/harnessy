import { ExternalLink, Play, Square } from "lucide-react";
import { useEffect, useRef } from "react";
import type { PreviewInfo } from "../lib/desktop";
import { isLive, previewLabel } from "../lib/preview";

interface Props {
  taskId: number;
  info: PreviewInfo;
  log: string[];
  onOpen: () => void;
  onStop: () => void;
  onRestart: () => void;
}

/** Ambiente de teste da tarefa: estado em texto, endereço, ações e o log do que sobe. */
export function PreviewPanel({ taskId, info, log, onOpen, onStop, onRestart }: Props) {
  const label = previewLabel(info);
  const live = isLive(info);
  const box = useRef<HTMLPreElement>(null);

  // Acompanha o fim do log, a menos que quem lê tenha rolado para cima.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `log.length` é o gatilho.
  useEffect(() => {
    const element = box.current;
    if (!element) return;
    const nearEnd = element.scrollHeight - element.scrollTop - element.clientHeight < 48;
    if (nearEnd) element.scrollTop = element.scrollHeight;
  }, [log.length]);

  return (
    <section className="preview-panel" aria-label={`Ambiente de teste da tarefa #${taskId}`}>
      <header>
        <h3>Ambiente de teste</h3>
        <span className={`chip preview-chip tone-${label.tone}`}>{label.text}</span>
        <span className="sr-only" role="status" aria-live="polite">
          Ambiente da tarefa {taskId}: {label.text}
        </span>
      </header>
      {info.url && (
        <p className="preview-url">
          Endereço <code>{info.url}</code>
        </p>
      )}
      <div className="preview-actions">
        <button
          type="button"
          className="button primary"
          disabled={info.state !== "running"}
          onClick={onOpen}
        >
          <ExternalLink size={14} aria-hidden="true" /> Abrir no navegador
        </button>
        {live ? (
          <button type="button" className="button" onClick={onStop}>
            <Square size={14} aria-hidden="true" /> Parar
          </button>
        ) : (
          <button type="button" className="button" onClick={onRestart}>
            <Play size={14} aria-hidden="true" /> Rodar de novo
          </button>
        )}
      </div>
      <pre
        ref={box}
        className="preview-log"
        // biome-ignore lint/a11y/noNoninteractiveTabindex: a área rolável precisa de foco para quem usa teclado.
        tabIndex={0}
        role="log"
        aria-label="Saída do ambiente de teste"
      >
        {log.length > 0 ? log.join("\n") : "Sem saída ainda."}
      </pre>
    </section>
  );
}
