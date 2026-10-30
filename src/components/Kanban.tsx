import { ArrowRight, Pencil, Trash2, TriangleAlert } from "lucide-react";
import { memo, useRef, useState } from "react";
import { type Block, LANES, neighbor, shortcutTarget, statusName } from "../lib/board";
import type { LocalTask, TaskStatus } from "../lib/desktop";
import "../styles/kanban.css";

interface Props {
  groups: Record<TaskStatus, LocalTask[]>;
  selectedId: number | null;
  /** Tarefas com run ativa, para o selo "agente trabalhando". */
  working: ReadonlySet<number>;
  /** Tarefas cuja última run ficou presa, com o motivo. */
  blocked: ReadonlyMap<number, Block>;
  /** Card recém-movido ou criado, destacado por um instante. */
  flashId: number | null;
  onSelect: (id: number) => void;
  onMove: (id: number, to: TaskStatus) => void;
  /** Abre a tarefa com o cursor no título. */
  onEdit: (id: number) => void;
  onDelete: (id: number) => void;
  onCreate: (title: string) => Promise<void>;
}

/** Tarefas do workspace em quatro raias. Mover: botão do card, teclado ou arrastar. */
export function Kanban({
  groups,
  selectedId,
  working,
  blocked,
  flashId,
  onSelect,
  onMove,
  onEdit,
  onDelete,
  onCreate,
}: Props) {
  const [over, setOver] = useState<TaskStatus | null>(null);
  // Em ref: começar a arrastar não deve re-renderizar todos os cards.
  const dragging = useRef<number | null>(null);

  return (
    <section className="kanban" aria-label="Tarefas por estado">
      {LANES.map((lane) => (
        // biome-ignore lint/a11y/noStaticElementInteractions: alvo de soltar do arrastar; o mesmo movimento existe no botão e no teclado do card.
        <div
          key={lane.status}
          className={`kanban-lane tone-${lane.status}${over === lane.status ? " is-over" : ""}`}
          onDragOver={(event) => {
            event.preventDefault();
            if (over !== lane.status) setOver(lane.status);
          }}
          onDragLeave={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(null);
          }}
          onDrop={(event) => {
            event.preventDefault();
            setOver(null);
            if (dragging.current !== null) onMove(dragging.current, lane.status);
            dragging.current = null;
          }}
        >
          <h3 className="lane-title">
            <span className="lane-dot" aria-hidden="true" />
            {lane.name}
            <span className="lane-count">{groups[lane.status].length}</span>
          </h3>
          {lane.status === "todo" && <Composer onCreate={onCreate} />}
          <ul className="lane-cards">
            {groups[lane.status].map((task) => (
              <KanbanCard
                key={task.id}
                task={task}
                selected={selectedId === task.id}
                working={working.has(task.id)}
                blocked={blocked.get(task.id)?.reason}
                flash={flashId === task.id}
                onSelect={onSelect}
                onMove={onMove}
                onEdit={onEdit}
                onDelete={onDelete}
                onDragStart={(id) => {
                  dragging.current = id;
                }}
              />
            ))}
          </ul>
          {groups[lane.status].length === 0 && <p className="lane-empty">{lane.empty}</p>}
        </div>
      ))}
    </section>
  );
}

function Composer({ onCreate }: { onCreate: (title: string) => Promise<void> }) {
  const [title, setTitle] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <form
      className="composer-inline"
      onSubmit={async (event) => {
        event.preventDefault();
        const value = title.trim();
        if (!value || busy) return;
        setBusy(true);
        await onCreate(value);
        setTitle("");
        setBusy(false);
      }}
    >
      <label className="sr-only" htmlFor="new-task-title">
        Título da nova tarefa
      </label>
      <input
        id="new-task-title"
        value={title}
        maxLength={300}
        placeholder="Nova tarefa… (n)"
        autoComplete="off"
        disabled={busy}
        onChange={(event) => setTitle(event.target.value)}
      />
    </form>
  );
}

interface CardProps {
  task: LocalTask;
  selected: boolean;
  working: boolean;
  blocked?: string;
  flash: boolean;
  onSelect: (id: number) => void;
  onMove: (id: number, to: TaskStatus) => void;
  onEdit: (id: number) => void;
  onDelete: (id: number) => void;
  onDragStart: (id: number) => void;
}

/** Memoizado: mover um card re-renderiza só ele e as duas raias envolvidas. */
const KanbanCard = memo(function KanbanCard({
  task,
  selected,
  working,
  blocked,
  flash,
  onSelect,
  onMove,
  onEdit,
  onDelete,
  onDragStart,
}: CardProps) {
  const next = neighbor(task.status, 1);
  return (
    <li
      className={`kanban-card${selected ? " is-selected" : ""}${flash ? " is-flash" : ""}`}
      data-card={task.id}
      draggable
      onDragStart={(event) => {
        event.dataTransfer.setData("text/plain", String(task.id));
        event.dataTransfer.effectAllowed = "move";
        onDragStart(task.id);
      }}
    >
      <button
        type="button"
        className="card-open"
        aria-pressed={selected}
        onClick={() => onSelect(task.id)}
        onKeyDown={(event) => {
          if (event.metaKey || event.ctrlKey || event.altKey) return;
          const to = shortcutTarget(task.status, event.key);
          if (!to || to === task.status) return;
          event.preventDefault();
          onMove(task.id, to);
          // O card muda de raia e o React o remonta: devolve o foco ao teclado.
          requestAnimationFrame(() =>
            document.querySelector<HTMLElement>(`[data-card="${task.id}"] .card-open`)?.focus(),
          );
        }}
      >
        <strong>{task.title}</strong>
        <span className="card-meta">
          <span className="chip">#{task.id}</span>
          {blocked && (
            <span className="chip is-blocked" title={blocked}>
              <TriangleAlert size={11} aria-hidden="true" /> Bloqueada
            </span>
          )}
          {working && (
            <span className="card-working">
              <span className="pulse" aria-hidden="true" /> agente trabalhando
            </span>
          )}
        </span>
      </button>
      <div className="card-tools">
        <button
          type="button"
          className="card-tool"
          title="Editar"
          aria-label={`Editar "${task.title}"`}
          onClick={() => onEdit(task.id)}
        >
          <Pencil size={14} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="card-tool is-danger"
          title="Excluir"
          aria-label={`Excluir "${task.title}"`}
          onClick={() => onDelete(task.id)}
        >
          <Trash2 size={14} aria-hidden="true" />
        </button>
        {next && (
          <button
            type="button"
            className="card-tool"
            title={`Mover para ${statusName(next)} (])`}
            aria-label={`Mover "${task.title}" para ${statusName(next)}`}
            onClick={() => onMove(task.id, next)}
          >
            <ArrowRight size={14} aria-hidden="true" />
          </button>
        )}
      </div>
    </li>
  );
});

/** Esqueleto enquanto as tarefas carregam; ocupa o mesmo espaço do quadro real. */
export function KanbanSkeleton() {
  return (
    <section className="kanban" aria-busy="true" aria-label="Carregando tarefas">
      {LANES.map((lane, index) => (
        <div key={lane.status} className="kanban-lane">
          <div className="skeleton skeleton-title" />
          {index < 2 && <div className="skeleton skeleton-card" />}
        </div>
      ))}
    </section>
  );
}
