import { CircleCheck, CircleX, Minus } from "lucide-react";
import { useState } from "react";
import { agentOf } from "../lib/agents";
import type { Run } from "../lib/desktop";
import type { Runtime } from "../lib/roles";
import { conversations, isActive, stateLabel, timeAgo } from "../lib/runs";

/** Quantas conversas aparecem antes de "Mostrar mais". */
const RECENT = 6;

interface Props {
  runs: Run[];
  openRun: number | null;
  onOpen: (id: number) => void;
}

function Mark({ run }: { run: Run }) {
  if (isActive(run)) return <span className="pulse" aria-hidden="true" />;
  if (run.state === "completed") return <CircleCheck size={14} aria-hidden="true" />;
  if (run.state === "failed") return <CircleX size={14} aria-hidden="true" />;
  return <Minus size={14} aria-hidden="true" />;
}

function Row({
  run,
  count,
  selected,
  onOpen,
}: {
  run: Run;
  count: number;
  selected: boolean;
  onOpen: Props["onOpen"];
}) {
  return (
    <li>
      <button
        type="button"
        className={`run-row ${selected ? "is-selected" : ""} state-${run.state}`}
        aria-current={selected ? "true" : undefined}
        onClick={() => onOpen(run.id)}
      >
        <Mark run={run} />
        <span className="run-row-text">
          <strong>{run.title}</strong>
          <span>
            {stateLabel(run.state)}
            {count > 1 ? ` · ${count} runs` : ""} ·{" "}
            {agentOf(run.runtime as Runtime)?.label ?? run.runtime} · {timeAgo(run.startedAt)}
          </span>
        </span>
      </button>
    </li>
  );
}

/**
 * Uma linha por tarefa, com a run mais recente dela. A linha não muda de grupo quando a
 * run termina, e a conversa aberta fica sempre visível, mesmo fora das mais recentes.
 */
export function RunList({ runs, openRun, onOpen }: Props) {
  const [all, setAll] = useState(false);
  const list = conversations(runs);
  const openTask = runs.find((run) => run.id === openRun)?.taskId ?? null;
  const shown = all
    ? list
    : list.filter((item, at) => at < RECENT || item.latest.taskId === openTask);
  return (
    <nav className="run-list" aria-label="Conversas">
      {list.length === 0 && <p className="muted">Nenhuma conversa ainda.</p>}
      {list.length > 0 && (
        <>
          <h2 className="run-list-title">Conversas</h2>
          <ul>
            {shown.map(({ latest, count }) => (
              <Row
                key={latest.taskId}
                run={latest}
                count={count}
                selected={latest.taskId === openTask}
                onOpen={onOpen}
              />
            ))}
          </ul>
          {list.length > RECENT && (
            <button type="button" className="link-button" onClick={() => setAll(!all)}>
              {all ? "Mostrar menos" : `Mostrar mais (${list.length - shown.length})`}
            </button>
          )}
        </>
      )}
    </nav>
  );
}
