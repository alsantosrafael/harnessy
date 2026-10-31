import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { accessLabel, extraActions, originLabel, unavailableReason } from "../lib/actions";
import { type AgentChoice, agentOf, modelName } from "../lib/agents";
import { blockHelp, blockTitle } from "../lib/runs";
import { type Block, LANES, statusName } from "../lib/board";
import { AgentPicker } from "./AgentPicker";
import { NewActionForm } from "./NewActionForm";
import {
  type ActionInfo,
  approveAction,
  IMPLEMENT_ACTION,
  type Run,
  listActions,
  previewPrompt,
  saveTask,
  taskHistory,
  type LocalTask,
  type TaskActor,
  type TaskEvent,
  type TaskStatus,
} from "../lib/desktop";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const ACTORS: Record<TaskActor, string> = { user: "Você", system: "Harnessy" };

interface Props {
  task: LocalTask;
  /** Muda a cada `task-changed` desta tarefa, para recarregar o histórico. */
  historyTick: number;
  /** Bloqueio da última run desta tarefa, se houver. */
  block?: Block;
  choice: AgentChoice;
  onChoice: (choice: AgentChoice) => void;
  canStart: boolean;
  starting: boolean;
  onMove: (id: number, to: TaskStatus) => void;
  onSaved: (task: LocalTask) => void;
  /** Run concluída do passo anterior, de onde este passo pode continuar. */
  chainFrom: Run | null;
  /** Título do passo anterior, para o texto da opção. */
  previousTitle: string;
  /** Inicia a run com a Ação escolhida; sem Ação, usa a principal (implementar). */
  onStart: (id: number, action?: string, continueFrom?: number | null) => void;
  onResolve: (runId: number) => void;
  /** Implementar é o próximo passo da tarefa; senão é um recomeço e não vira o botão principal. */
  implementIsNext: boolean;
  onDirty: (dirty: boolean) => void;
  onClose: () => void;
  /** Continuar da run do passo anterior (escolha compartilhada com os comandos rápidos). */
  useChain: boolean;
  onUseChain: (on: boolean) => void;
}

/** Edição, estado, prompt do agente e histórico de uma tarefa. Use `key={task.id}`. */
export function TaskDetail({
  task,
  historyTick,
  block,
  chainFrom,
  previousTitle,
  choice,
  onChoice,
  canStart,
  starting,
  onMove,
  onSaved,
  onStart,
  onResolve,
  implementIsNext,
  onDirty,
  onClose,
  useChain,
  onUseChain,
}: Props) {
  const [form, setForm] = useState({
    title: task.title,
    description: task.description,
    criteria: task.criteria,
  });
  const [busy, setBusy] = useState(false);
  const continueFrom = chainFrom && useChain ? chainFrom.id : null;
  const [error, setError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [history, setHistory] = useState<TaskEvent[]>([]);
  const [actions, setActions] = useState<ActionInfo[]>([]);
  // Recarrega o catálogo depois de criar ou confirmar uma Ação.
  const [actionsTick, setActionsTick] = useState(0);
  const primary = actions.find((action) => action.id === IMPLEMENT_ACTION);
  const primaryReason = primary ? unavailableReason(primary) : null;
  const dirty =
    form.title !== task.title ||
    form.description !== task.description ||
    form.criteria !== task.criteria;

  useEffect(() => onDirty(dirty), [dirty, onDirty]);
  // Ao desmontar (outra tarefa ou fechar), o aviso de "não salvo" deixa de valer.
  useEffect(() => () => onDirty(false), [onDirty]);

  // `revision` muda ao salvar o conteúdo, que é o que entra no prompt.
  // biome-ignore lint/correctness/useExhaustiveDependencies: o prompt depende do conteúdo salvo, não do estado.
  useEffect(() => {
    let alive = true;
    previewPrompt(task.workspaceId, task.id)
      .then((text) => alive && setPrompt(text))
      .catch(() => alive && setPrompt(""));
    return () => {
      alive = false;
    };
  }, [task.workspaceId, task.id, task.revision]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `actionsTick` é o gatilho de recarga.
  useEffect(() => {
    let alive = true;
    listActions(task.workspaceId)
      .then((list) => alive && setActions(list))
      .catch(() => alive && setActions([]));
    return () => {
      alive = false;
    };
  }, [task.workspaceId, actionsTick]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `historyTick` é o gatilho de recarga.
  useEffect(() => {
    let alive = true;
    taskHistory(task.id)
      .then((rows) => alive && setHistory(rows))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [task.id, historyTick]);

  return (
    <div className="task-detail">
      <div className="detail-col">
        <form
          className="local-form"
          onSubmit={async (event) => {
            event.preventDefault();
            setBusy(true);
            setError("");
            try {
              const saved = await saveTask({
                ...form,
                workspaceId: task.workspaceId,
                id: task.id,
                revision: task.revision,
              });
              setForm({
                title: saved.title,
                description: saved.description,
                criteria: saved.criteria,
              });
              onSaved(saved);
            } catch (cause) {
              setError(message(cause));
            } finally {
              setBusy(false);
            }
          }}
        >
          <div className="detail-header">
            <h2>Tarefa #{task.id}</h2>
            <button
              type="button"
              className="icon-button"
              title="Fechar"
              aria-label="Fechar tarefa"
              onClick={onClose}
            >
              <X size={16} aria-hidden="true" />
            </button>
          </div>
          {block && (
            <div className="block-banner" role="status">
              <strong>{blockTitle(block.state)}</strong>
              <p>{block.reason}</p>
              <p className="muted">{blockHelp(block.state)}</p>
              <div className="block-actions">
                {block.state === "no_changes" && task.status !== "done" && (
                  <button
                    type="button"
                    className="button primary"
                    onClick={() => onMove(task.id, "done")}
                  >
                    Concluir tarefa
                  </button>
                )}
                <button type="button" className="button" onClick={() => onResolve(block.runId)}>
                  Dispensar aviso
                </button>
              </div>
            </div>
          )}
          <fieldset className="status-group">
            <legend className="sr-only">Estado da tarefa</legend>
            {LANES.map((lane) => (
              <button
                key={lane.status}
                type="button"
                className={`status-pill tone-${lane.status}`}
                aria-pressed={task.status === lane.status}
                onClick={() => onMove(task.id, lane.status)}
              >
                <span className="lane-dot" aria-hidden="true" />
                {lane.name}
              </button>
            ))}
          </fieldset>
          <label>
            Título
            <input
              id="task-title"
              required
              maxLength={300}
              value={form.title}
              disabled={busy}
              onChange={(event) => setForm({ ...form, title: event.target.value })}
            />
          </label>
          <label>
            Descrição
            <textarea
              rows={5}
              maxLength={100000}
              value={form.description}
              disabled={busy}
              onChange={(event) => setForm({ ...form, description: event.target.value })}
            />
          </label>
          <label>
            Critérios de aceite
            <textarea
              rows={4}
              maxLength={100000}
              value={form.criteria}
              disabled={busy}
              onChange={(event) => setForm({ ...form, criteria: event.target.value })}
            />
          </label>
          {error && (
            <p className="local-error" role="alert">
              {error}
            </p>
          )}
          <div className="form-footer">
            <span role="status" className="muted">
              {dirty ? "Alterações não salvas" : "Salva"}
            </span>
            <button className="button" disabled={busy || !dirty || !form.title.trim()}>
              {busy ? "Salvando…" : "Salvar"}
            </button>
          </div>
        </form>
      </div>

      <div className="detail-col">
        <section className="run-bar" aria-label="Implementar">
          <AgentPicker choice={choice} onChange={onChoice} defaultOpen />
          {chainFrom && (
            <label className="chain-option">
              <input
                type="checkbox"
                checked={useChain}
                onChange={(event) => onUseChain(event.target.checked)}
              />
              Continuar das alterações de “{previousTitle}” (branch <code>{chainFrom.branch}</code>)
            </label>
          )}
          <button
            type="button"
            className={implementIsNext ? "button primary" : "button"}
            disabled={!canStart || starting || dirty || primaryReason !== null}
            onClick={() => onStart(task.id, undefined, continueFrom)}
          >
            {starting
              ? "Iniciando…"
              : `${implementIsNext ? "Implementar" : "Implementar de novo"} com ${agentOf(choice.runtime)?.label ?? choice.runtime} · ${modelName(choice.runtime, choice.model)}`}
          </button>
          {!implementIsNext && (
            <p className="muted">
              Já existe uma implementação. Para mudar algo, peça um ajuste na conversa; recomeçar do
              zero é só se você quiser descartar o caminho atual.
            </p>
          )}
          {dirty && <p className="muted">Salve a tarefa antes de implementar.</p>}
          {primaryReason && <p className="local-error">{primaryReason}</p>}
        </section>

        <details className="more-actions">
          <summary>Mais ações</summary>
          {extraActions(actions).length > 0 && (
            <ul>
              {extraActions(actions).map((action) => {
                const reason = unavailableReason(action);
                return (
                  <li key={action.id}>
                    <button
                      type="button"
                      className="button"
                      disabled={!canStart || starting || dirty || reason !== null}
                      onClick={() => onStart(task.id, action.id, continueFrom)}
                    >
                      {action.nome}
                    </button>
                    <p className="more-actions-note">{action.descricao}</p>
                    <small className="more-actions-note">
                      {[originLabel(action.origem), accessLabel(action.acesso)]
                        .filter(Boolean)
                        .join(" · ")}
                    </small>
                    {reason && <p className="local-error">{reason}</p>}
                    {action.pendente && (
                      <button
                        type="button"
                        className="button"
                        onClick={() =>
                          approveAction(task.workspaceId, action.id)
                            .then(() => setActionsTick((tick) => tick + 1))
                            .catch((cause) => setError(message(cause)))
                        }
                      >
                        Confirmar Ação do repositório
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          <details>
            <summary>Nova Ação</summary>
            <NewActionForm onCreated={() => setActionsTick((tick) => tick + 1)} />
          </details>
        </details>

        <details>
          <summary>Prompt que o agente receberá</summary>
          <pre className="run-log">{prompt || "Carregando…"}</pre>
        </details>

        <section aria-label="Histórico">
          <h3 className="section-title">Histórico</h3>
          <ol className="history">
            {history.map((event) => (
              <li key={event.seq} className={`actor-${event.actor}`}>
                <span className="history-who">{ACTORS[event.actor] ?? "Harnessy"}</span>
                <span>
                  {event.fromStatus === null
                    ? `criou em ${statusName(event.toStatus)}`
                    : event.fromStatus === event.toStatus
                      ? `anotou em ${statusName(event.toStatus)}`
                      : `${statusName(event.fromStatus)} → ${statusName(event.toStatus)}`}
                </span>
                <time dateTime={new Date(event.at * 1000).toISOString()}>
                  {new Date(event.at * 1000).toLocaleString("pt-BR", {
                    day: "2-digit",
                    month: "2-digit",
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </time>
                {event.note && <q className="history-note">{event.note}</q>}
              </li>
            ))}
          </ol>
        </section>
      </div>
    </div>
  );
}
