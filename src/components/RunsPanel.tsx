import { CircleCheck, Info, TriangleAlert } from "lucide-react";
import { useState } from "react";
import {
  cancelRun,
  continueRun,
  createSteps,
  type LocalTask,
  type Run,
  type RunEvent,
  type StepProposal,
} from "../lib/desktop";
import {
  canTryNextModel,
  isActive,
  proofLabel,
  type ProofTone,
  reviewLabel,
  stateLabel,
  stepsFrom,
} from "../lib/runs";

const proofIcons: Record<ProofTone, typeof Info> = {
  positive: CircleCheck,
  alert: TriangleAlert,
  muted: Info,
};

interface Props {
  runs: Run[];
  openRun: number | null;
  events: RunEvent[];
  onOpen: (id: number) => void;
  onError: (text: string) => void;
  /** Tarefas criadas a partir de passos confirmados. */
  onStepsCreated: (tasks: LocalTask[]) => void;
}

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** Veredito da contraprova com ícone e texto; nada quando a etapa não rodou. */
function ProofLine({ proof }: { proof: string | null }) {
  const label = proofLabel(proof);
  if (!label) return null;
  const Icon = proofIcons[label.tone];
  return (
    <p className={`run-proof is-${label.tone}`}>
      <Icon aria-hidden="true" size={16} /> {label.text}
    </p>
  );
}

/** Veredito do revisor independente, com ícone e texto; nada sem revisão. */
function ReviewLine({ run }: { run: Run }) {
  const label = reviewLabel(run);
  if (!label) return null;
  const Icon = proofIcons[label.tone];
  return (
    <p className={`run-proof is-${label.tone}`}>
      <Icon aria-hidden="true" size={16} /> {label.text}
    </p>
  );
}

/** Passos propostos, editáveis; confirmar cria as tarefas encadeadas. */
function StepsProposal({
  run,
  proposed,
  onCreated,
  onError,
}: {
  run: Run;
  proposed: StepProposal[];
  onCreated: (tasks: LocalTask[]) => void;
  onError: (text: string) => void;
}) {
  // Chave estável por passo: remover um passo não embaralha os campos dos outros.
  const [steps, setSteps] = useState(() => proposed.map((step, key) => ({ ...step, key })));
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const update = (at: number, patch: Partial<StepProposal>) =>
    setSteps((current) => current.map((step, i) => (i === at ? { ...step, ...patch } : step)));
  if (done) return <p role="status">Passos criados em A fazer.</p>;
  return (
    <form
      className="local-form steps-proposal"
      onSubmit={(event) => {
        event.preventDefault();
        setBusy(true);
        createSteps(
          run.workspaceId,
          run.taskId,
          steps.map(({ title, criteria }) => ({ title, criteria })),
        )
          .then((tasks) => {
            setDone(true);
            onCreated(tasks);
          })
          .catch((cause) => onError(message(cause)))
          .finally(() => setBusy(false));
      }}
    >
      <h4>Passos propostos</h4>
      <p>
        Edite ou remova antes de criar. Cada passo vira uma tarefa que pode continuar do anterior.
      </p>
      <ol>
        {steps.map((step, at) => (
          <li key={step.key}>
            <label>
              Título do passo {at + 1}
              <input
                value={step.title}
                maxLength={300}
                disabled={busy}
                onChange={(event) => update(at, { title: event.target.value })}
              />
            </label>
            <label>
              Critérios do passo {at + 1}
              <textarea
                rows={2}
                value={step.criteria}
                disabled={busy}
                onChange={(event) => update(at, { criteria: event.target.value })}
              />
            </label>
            <button
              type="button"
              className="button"
              disabled={busy || steps.length === 1}
              onClick={() => setSteps((current) => current.filter((_, i) => i !== at))}
            >
              Remover passo {at + 1}
            </button>
          </li>
        ))}
      </ol>
      <button
        className="button primary"
        disabled={busy || steps.some((step) => !step.title.trim())}
      >
        {busy ? "Criando…" : `Criar ${steps.length} tarefa(s)`}
      </button>
    </form>
  );
}

/** Runs do workspace e o log da selecionada. */
export function RunsPanel({ runs, openRun, events, onOpen, onError, onStepsCreated }: Props) {
  const shown = runs.find((run) => run.id === openRun) ?? null;
  return (
    <section aria-label="Runs">
      <h3>Runs</h3>
      {runs.length === 0 && <div className="local-empty">Nenhuma run neste workspace.</div>}
      <div className="local-columns">
        <div>
          {runs.map((run) => (
            <button
              key={run.id}
              type="button"
              className={`local-task ${openRun === run.id ? "is-selected" : ""}`}
              aria-pressed={openRun === run.id}
              onClick={() => onOpen(run.id)}
            >
              <strong>{run.title}</strong>
              <span>
                <span className="badge">{stateLabel(run.state)}</span>
                {run.branch ? ` ${run.branch}` : ""}
              </span>
            </button>
          ))}
        </div>
        <div className="task-editor">
          {shown ? (
            <>
              <h3>
                {shown.title} · {stateLabel(shown.state)}
              </h3>
              {shown.summary && <p>{shown.summary}</p>}
              <ProofLine proof={shown.proof} />
              <ReviewLine run={shown} />
              {shown.state === "completed" && stepsFrom(events) && (
                <StepsProposal
                  key={shown.id}
                  run={shown}
                  proposed={stepsFrom(events) ?? []}
                  onCreated={onStepsCreated}
                  onError={onError}
                />
              )}
              {shown.branch && !isActive(shown) && (
                <p>
                  Branch <code>{shown.branch}</code> no repositório do workspace. Revise e teste
                  antes do merge.
                </p>
              )}
              {canTryNextModel(shown) && (
                <div className="run-next-model">
                  <p>
                    As verificações não passaram com {shown.lastModel}. Um ciclo com{" "}
                    {shown.nextModel} continua a partir das alterações já feitas.
                  </p>
                  <button
                    type="button"
                    className="button"
                    onClick={() => continueRun(shown.id).catch((cause) => onError(message(cause)))}
                  >
                    Tentar com {shown.nextModel}
                  </button>
                </div>
              )}
              {isActive(shown) && (
                <button
                  type="button"
                  className="button"
                  onClick={() => cancelRun(shown.id).catch((cause) => onError(message(cause)))}
                >
                  Cancelar run
                </button>
              )}
              <pre className="run-log" aria-live="polite">
                {events.map((event) => `[${event.kind}] ${event.text}`).join("\n") ||
                  "Sem eventos ainda."}
              </pre>
            </>
          ) : (
            <p>Selecione uma run para acompanhar o log.</p>
          )}
        </div>
      </div>
    </section>
  );
}
