import {
  CircleCheck,
  Copy,
  MessageSquare,
  Play,
  RotateCcw,
  SearchCheck,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import { type QuickId, type QuickState } from "../lib/quick";

const icons: Record<QuickId, LucideIcon> = {
  implement: Play,
  review: SearchCheck,
  conversation: MessageSquare,
  branch: Copy,
  finish: CircleCheck,
  reopen: RotateCcw,
};

interface Props {
  taskId: number;
  states: QuickState[];
  /** Tarefa e run desta coluna, para o leitor de tela saber de qual se trata. */
  title: string;
  onRun: (id: QuickId) => void;
  onDelete: () => void;
}

/** Comandos rápidos da tarefa selecionada, ao lado do quadro. Texto sempre visível. */
export function QuickActions({ taskId, states, title, onRun, onDelete }: Props) {
  return (
    <nav className="quick-actions" aria-label={`Comandos rápidos da tarefa #${taskId}`}>
      <h2 className="quick-title" title={title}>
        Tarefa #{taskId}
      </h2>
      <ul>
        {states.map((state) => {
          const Icon = icons[state.id];
          return (
            <li key={state.id}>
              <button
                type="button"
                className={`quick-button${state.id === "implement" ? " is-primary" : ""}`}
                disabled={state.reason !== null}
                aria-describedby={state.reason ? `quick-why-${state.id}` : undefined}
                onClick={() => onRun(state.id)}
              >
                <Icon size={15} aria-hidden="true" />
                {state.label}
              </button>
              {state.reason && (
                <small id={`quick-why-${state.id}`} className="quick-why">
                  {state.reason}
                </small>
              )}
            </li>
          );
        })}
      </ul>
      <button type="button" className="quick-button is-danger" onClick={onDelete}>
        <Trash2 size={15} aria-hidden="true" />
        Excluir tarefa
      </button>
    </nav>
  );
}
