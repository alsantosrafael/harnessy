import {
  CircleCheck,
  Copy,
  GitMerge,
  GitPullRequest,
  MonitorPlay,
  MessageSquare,
  Play,
  Sparkles,
  RotateCcw,
  SearchCheck,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import type { Progress } from "../lib/progress";
import type { NextAction, QuickId, QuickState } from "../lib/quick";

const icons: Record<QuickId, LucideIcon> = {
  refine: Sparkles,
  implement: Play,
  review: SearchCheck,
  preview: MonitorPlay,
  publish: GitPullRequest,
  merge: GitMerge,
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
  /** A ação que o laço pede agora; vira o único botão principal. */
  next: NextAction | null;
  /** O agente está trabalhando: não há próximo passo, só esperar. */
  working: boolean;
  /** Etapa, atividade e tempo da run em andamento, para o usuário acompanhar sem sair do quadro. */
  progress: Progress | null;
  onCancel: () => void;
  onOpenChat: () => void;
  onRun: (id: QuickId) => void;
  onDelete: () => void;
}

/** Comandos rápidos da tarefa selecionada, ao lado do quadro. Texto sempre visível. */
export function QuickActions({
  taskId,
  states,
  title,
  next,
  working,
  progress,
  onCancel,
  onOpenChat,
  onRun,
  onDelete,
}: Props) {
  const others = states.filter((state) => state.id !== next?.id);
  const NextIcon = next ? icons[next.id] : null;
  return (
    <nav className="quick-actions" aria-label={`Comandos rápidos da tarefa #${taskId}`}>
      <h2 className="quick-title" title={title}>
        Tarefa #{taskId}
      </h2>
      {next && NextIcon && (
        <div className="next-action">
          <span className="next-kicker">Próximo passo</span>
          <button
            type="button"
            className="quick-button is-primary is-next"
            aria-keyshortcuts="A"
            aria-describedby="next-why"
            onClick={() => onRun(next.id)}
          >
            <NextIcon size={16} aria-hidden="true" />
            {next.label}
            <kbd>A</kbd>
          </button>
          <small className="quick-why" id="next-why">
            {next.why}
          </small>
        </div>
      )}
      {!next && working && (
        <div className="agent-progress">
          <p className="agent-progress-head">
            <span className="pulse" aria-hidden="true" />
            <strong>{progress?.phase ?? "O agente está trabalhando"}</strong>
            {progress && <span className="agent-elapsed">{progress.elapsed}</span>}
          </p>
          {progress?.activity && <p className="agent-activity">{progress.activity}</p>}
          <p className="muted">
            Pode continuar aqui ou usar outros aplicativos: o Harnessy avisa quando precisar de
            você.
          </p>
          <div className="agent-progress-actions">
            <button type="button" className="button" onClick={onOpenChat}>
              Ver conversa
            </button>
            <button type="button" className="button" onClick={onCancel}>
              Cancelar
            </button>
          </div>
          <span className="sr-only" role="status">
            {progress?.phase}
          </span>
        </div>
      )}
      {/* Sem próximo passo, as ações ficam à vista; com ele, recolhidas para não competir. */}
      <details className="quick-more" key={next?.id ?? "none"} open={!next && !working}>
        <summary>Mais ações</summary>
        <ul>
          {others.map((state) => {
            const Icon = icons[state.id];
            return (
              <li key={state.id}>
                <button
                  type="button"
                  className="quick-button"
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
      </details>
      <button type="button" className="quick-button is-danger" onClick={onDelete}>
        <Trash2 size={15} aria-hidden="true" />
        Excluir tarefa
      </button>
    </nav>
  );
}
