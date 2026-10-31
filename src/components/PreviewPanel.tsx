import { Check, Copy, ExternalLink, Play, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { PreviewInfo } from "../lib/desktop";
import { hintFor, isLive, logKind, previewLabel } from "../lib/preview";

/** Marcador que o Harnessy troca por uma porta livre; aparece como código na dica. */
const PORT_MARK = "{port}";

interface Props {
  taskId: number;
  info: PreviewInfo;
  log: string[];
  onOpen: () => void;
  onStop: () => void;
  onRestart: () => void;
  /** Abre o diálogo para trocar o comando de subir o projeto. */
  onEditCommand: () => void;
}

/** Ambiente de teste da tarefa: estado em texto, endereço, ações e o log do que sobe. */
export function PreviewPanel({
  taskId,
  info,
  log,
  onOpen,
  onStop,
  onRestart,
  onEditCommand,
}: Props) {
  const hint = hintFor(info, log);
  const label = previewLabel(info);
  const live = isLive(info);
  const box = useRef<HTMLPreElement>(null);
  const [copied, setCopied] = useState(false);
  const copy = () =>
    navigator.clipboard
      .writeText(log.join("\n"))
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1800);
      })
      .catch(() => undefined);

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
      {hint && (
        <p className="preview-hint" role="status">
          {hint.includes(PORT_MARK) ? (
            <>
              {hint.slice(0, hint.indexOf(PORT_MARK))}
              <code>{PORT_MARK}</code>
              {hint.slice(hint.indexOf(PORT_MARK) + PORT_MARK.length)}
            </>
          ) : (
            hint
          )}{" "}
          <button type="button" className="link-button" onClick={onEditCommand}>
            Editar comando
          </button>
        </p>
      )}
      <div className="log-box">
        <div className="log-bar">
          <span className="log-bar-title">Saída</span>
          <span>
            {log.length} {log.length === 1 ? "linha" : "linhas"}
            {" · "}
            <button
              type="button"
              className="log-copy"
              disabled={log.length === 0}
              onClick={copy}
              aria-label="Copiar a saída do ambiente de teste"
            >
              {copied ? (
                <Check size={11} aria-hidden="true" />
              ) : (
                <Copy size={11} aria-hidden="true" />
              )}{" "}
              {copied ? "Copiado" : "Copiar"}
            </button>
          </span>
        </div>
        <pre
          ref={box}
          className="preview-log"
          // biome-ignore lint/a11y/noNoninteractiveTabindex: a área rolável precisa de foco para quem usa teclado.
          tabIndex={0}
          role="log"
          aria-label="Saída do ambiente de teste"
        >
          {log.length > 0 ? (
            log.map((line, index) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: as linhas do log só crescem no fim.
              <span key={index} className={`log-line is-${logKind(line)}`}>
                {line}
                {"\n"}
              </span>
            ))
          ) : (
            <span className="log-empty">Sem saída ainda.</span>
          )}
        </pre>
      </div>
    </section>
  );
}
