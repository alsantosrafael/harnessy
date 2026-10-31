import { useEffect, useMemo, useState } from "react";
import { type FileDiff, type RunDiff, runDiff } from "../lib/desktop";
import { diffSummary, findingsByLine, parsePatch } from "../lib/diff";
import type { Finding } from "../lib/feed";

interface Props {
  runId: number;
  taskId: number;
  /** Achados da revisão (os que bloqueiam e as sugestões), para aparecerem na linha deles. */
  findings: Finding[];
  /** Abre já ao montar: na raia de Revisão, ler o que mudou é o centro do trabalho. */
  defaultOpen?: boolean;
}

const STATUS_LABEL: Record<FileDiff["status"], string> = {
  modified: "alterado",
  added: "novo",
  deleted: "removido",
  renamed: "renomeado",
};

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

/**
 * O que a run mudou, para o humano ler antes de aprovar. Só leitura: carrega ao abrir,
 * mostra um arquivo por vez e põe cada achado da revisão na linha a que ele se refere.
 */
export function ChangesPanel({ runId, taskId, findings, defaultOpen = false }: Props) {
  const [open, setOpen] = useState(defaultOpen);
  const [diff, setDiff] = useState<RunDiff | null>(null);
  const [error, setError] = useState("");
  const [path, setPath] = useState<string | null>(null);

  useEffect(() => {
    if (!open || diff) return;
    let alive = true;
    runDiff(runId)
      .then((result) => {
        if (!alive) return;
        setDiff(result);
        // Abre pelo primeiro arquivo que a revisão comentou; senão, pelo primeiro.
        const flagged = result.files.find((file) => findings.some((f) => f.arquivo === file.path));
        setPath((flagged ?? result.files[0])?.path ?? null);
      })
      .catch((cause) => alive && setError(message(cause)));
    return () => {
      alive = false;
    };
  }, [open, diff, runId, findings]);

  const file = diff?.files.find((item) => item.path === path) ?? null;
  const hunks = useMemo(() => (file ? parsePatch(file.patch) : []), [file]);
  const anchored = useMemo(
    () => (file ? findingsByLine(findings, file.path) : new Map()),
    [file, findings],
  );

  return (
    <details
      className="changes-panel"
      open={open}
      onToggle={(event) => setOpen((event.currentTarget as HTMLDetailsElement).open)}
    >
      <summary>
        Alterações da run
        {diff && (
          <span className="changes-summary">
            {diffSummary(diff.files.length, diff.additions, diff.deletions)}
          </span>
        )}
      </summary>
      {open && !diff && !error && <p className="muted">Lendo as alterações…</p>}
      {error && (
        <p className="local-error" role="alert">
          {error}
        </p>
      )}
      {diff && diff.files.length === 0 && <p className="muted">A branch não tem alterações.</p>}
      {diff && diff.files.length > 0 && (
        <div className="changes-body">
          <ul className="changes-files" aria-label={`Arquivos alterados pela tarefa #${taskId}`}>
            {diff.files.map((item) => {
              const flagged = findings.filter((f) => f.arquivo === item.path).length;
              return (
                <li key={item.path}>
                  <button
                    type="button"
                    className="changes-file"
                    aria-pressed={item.path === path}
                    onClick={() => setPath(item.path)}
                  >
                    <span className="changes-path">{item.path}</span>
                    <span className="changes-counts">
                      <span className="changes-status">{STATUS_LABEL[item.status]}</span>
                      <span>+{item.additions}</span> <span>−{item.deletions}</span>
                      {flagged > 0 && (
                        <span className="changes-flag">
                          {flagged} {flagged === 1 ? "achado" : "achados"}
                        </span>
                      )}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
          {diff.truncated && (
            <p className="muted">Há mais alterações do que o app mostra; veja o restante no git.</p>
          )}
          {file && (
            <section
              className="changes-patch"
              // biome-ignore lint/a11y/noNoninteractiveTabindex: a área rolável precisa de foco para quem usa teclado.
              tabIndex={0}
              aria-label={`Alterações em ${file.path}`}
            >
              {file.binary && <p className="muted">Arquivo binário: sem prévia.</p>}
              {file.truncated && (
                <p className="muted">Este arquivo é grande: a prévia termina antes do fim.</p>
              )}
              {hunks.map((hunk) => (
                <table className="hunk" key={hunk.header}>
                  <caption>{hunk.header}</caption>
                  <tbody>
                    {hunk.lines.map((line, index) => {
                      const notes = line.newNo !== null ? anchored.get(line.newNo) : undefined;
                      return (
                        <HunkRow
                          // biome-ignore lint/suspicious/noArrayIndexKey: as linhas de um hunk são estáticas.
                          key={index}
                          line={line}
                          notes={notes}
                        />
                      );
                    })}
                  </tbody>
                </table>
              ))}
            </section>
          )}
        </div>
      )}
    </details>
  );
}

function HunkRow({
  line,
  notes,
}: {
  line: ReturnType<typeof parsePatch>[number]["lines"][number];
  notes?: Finding[];
}) {
  const sign = line.kind === "add" ? "+" : line.kind === "del" ? "−" : " ";
  return (
    <>
      <tr className={`diff-line is-${line.kind}`}>
        <td className="diff-no">{line.oldNo ?? ""}</td>
        <td className="diff-no">{line.newNo ?? ""}</td>
        <td
          className="diff-sign"
          aria-label={
            line.kind === "add" ? "adicionada" : line.kind === "del" ? "removida" : "inalterada"
          }
        >
          {sign}
        </td>
        <td className="diff-text">
          <code>{line.text}</code>
        </td>
      </tr>
      {notes?.map((note) => (
        <tr className="diff-note" key={`${note.arquivo}:${note.linha}:${note.texto}`}>
          <td colSpan={4}>
            <strong>{note.severidade === "bloqueia" ? "Bloqueia" : "Sugere"}</strong> {note.texto}
          </td>
        </tr>
      ))}
    </>
  );
}
