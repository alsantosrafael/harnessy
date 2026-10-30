import { CircleCheck, CircleX, Minus } from "lucide-react";
import { useState } from "react";
import { agentOf } from "../lib/agents";
import type { Run } from "../lib/desktop";
import type { Runtime } from "../lib/roles";
import { isActive, stateLabel, timeAgo } from "../lib/runs";

/** Quantas runs anteriores aparecem antes de "Mostrar mais". */
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

function Row({ run, selected, onOpen }: { run: Run; selected: boolean; onOpen: Props["onOpen"] }) {
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
            {stateLabel(run.state)} · {agentOf(run.runtime as Runtime)?.label ?? run.runtime} ·{" "}
            {timeAgo(run.startedAt)}
          </span>
        </span>
      </button>
    </li>
  );
}

/** Runs do workspace: as ativas no topo, as anteriores logo abaixo e recolhidas. */
export function RunList({ runs, openRun, onOpen }: Props) {
  const [all, setAll] = useState(false);
  const active = runs.filter(isActive);
  const past = runs.filter((run) => !isActive(run));
  const shownPast = all ? past : past.slice(0, RECENT);
  return (
    <nav className="run-list" aria-label="Runs">
      {runs.length === 0 && <p className="muted">Nenhuma run ainda.</p>}
      {active.length > 0 && (
        <>
          <h2 className="run-list-title">Em andamento</h2>
          <ul>
            {active.map((run) => (
              <Row key={run.id} run={run} selected={run.id === openRun} onOpen={onOpen} />
            ))}
          </ul>
        </>
      )}
      {past.length > 0 && (
        <>
          <h2 className="run-list-title">Anteriores</h2>
          <ul>
            {shownPast.map((run) => (
              <Row key={run.id} run={run} selected={run.id === openRun} onOpen={onOpen} />
            ))}
          </ul>
          {past.length > RECENT && (
            <button type="button" className="link-button" onClick={() => setAll(!all)}>
              {all ? "Mostrar menos" : `Mostrar mais (${past.length - RECENT})`}
            </button>
          )}
        </>
      )}
    </nav>
  );
}
