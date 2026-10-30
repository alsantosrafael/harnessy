import { CircleCheck, Info, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import {
  cancelRun,
  continueRun,
  createSteps,
  type LocalTask,
  type Run,
  type RunEvent,
  type StepProposal,
} from "../lib/desktop";
import { type FeedItem, type ReviewPayload, buildFeed } from "../lib/feed";
import { agentOf, modelName } from "../lib/agents";
import type { Runtime } from "../lib/roles";
import {
  canTryNextModel,
  isActive,
  proofLabel,
  type ProofTone,
  stateLabel,
  stepsFrom,
} from "../lib/runs";

const proofIcons: Record<ProofTone, typeof Info> = {
  positive: CircleCheck,
  alert: TriangleAlert,
  muted: Info,
};

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

interface Props {
  run: Run;
  events: RunEvent[];
  onError: (text: string) => void;
  /** Passos confirmados viraram tarefas. */
  onStepsCreated: (tasks: LocalTask[]) => void;
}

const agentLabel = (runtime: string) => agentOf(runtime as Runtime)?.label ?? runtime;

/** Revisor em texto: o agente (`codex` ou um modelo do Claude Code) que emitiu o veredito. */
const reviewerName = (reviewer: string) =>
  reviewer === "codex"
    ? "Codex"
    : reviewer
      ? `Claude Code · ${modelName("claude", reviewer)}`
      : "Revisor";

const verdicts: Record<string, { text: string; tone: ProofTone }> = {
  approve: { text: "Aprovou", tone: "positive" },
  request_changes: { text: "Pediu mudanças", tone: "alert" },
  invalid: { text: "Resposta fora do formato; sem revisão", tone: "muted" },
};

function ReviewCard({ review }: { review: ReviewPayload }) {
  const verdict = verdicts[review.verdict] ?? verdicts.invalid;
  const Icon = proofIcons[verdict.tone];
  const findings = [
    ...review.blocking.map((finding) => ({ ...finding, label: "Bloqueia" })),
    ...review.suggestions.map((finding) => ({ ...finding, label: "Sugere" })),
  ];
  return (
    <section className="feed-review" aria-label="Revisão">
      <h3 className={`run-proof is-${verdict.tone}`}>
        <Icon aria-hidden="true" size={16} /> Revisão · {reviewerName(review.reviewer)} ·{" "}
        {verdict.text}
      </h3>
      {findings.length > 0 && (
        <ul>
          {findings.map((finding) => (
            <li key={`${finding.arquivo}:${finding.linha}:${finding.texto}`}>
              <span className={`finding-tag ${finding.label === "Bloqueia" ? "is-blocking" : ""}`}>
                {finding.label}
              </span>
              <code>
                {finding.arquivo}:{finding.linha}
              </code>
              <p>{finding.texto}</p>
            </li>
          ))}
        </ul>
      )}
      {review.discarded > 0 && (
        <p className="muted">{review.discarded} achado(s) fora do diff descartado(s).</p>
      )}
    </section>
  );
}

function FeedEntry({ item, agent }: { item: FeedItem; agent: string }) {
  switch (item.kind) {
    case "message":
      return (
        <div className="feed-message">
          <span className="feed-who">{agent}</span>
          <p>{item.text}</p>
        </div>
      );
    case "tools":
      return (
        <details className="feed-group">
          <summary>
            {item.lines.length} ação(ões) · <code>{item.lines[item.lines.length - 1]}</code>
          </summary>
          <ul>
            {item.lines.map((line, at) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: linhas repetidas; a ordem é estável.
              <li key={at}>
                <code>{line}</code>
              </li>
            ))}
          </ul>
        </details>
      );
    case "logs":
      return (
        <details className="feed-group is-technical">
          <summary>Logs técnicos ({item.lines.length})</summary>
          <pre>{item.lines.join("\n")}</pre>
        </details>
      );
    case "review":
      return <ReviewCard review={item.review} />;
    case "denied":
      return (
        <p className="feed-denied">
          <TriangleAlert size={13} aria-hidden="true" /> Negado: <code>{item.text}</code>
        </p>
      );
    case "result":
      return <p className="feed-result">{item.text}</p>;
    default:
      return <p className="feed-status">{item.text}</p>;
  }
}

/** Conversa da run: o que o agente disse e fez, a revisão e as ações possíveis agora. */
export function RunFeed({ run, events, onError, onStepsCreated }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  const feed = buildFeed(events);
  const active = isActive(run);
  const proposed = run.state === "completed" ? stepsFrom(events) : null;
  const agent = agentLabel(run.runtime);

  // Acompanha o fim da conversa, a menos que a pessoa tenha rolado para ler algo acima.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `events` e `run.state` disparam o acompanhamento.
  useEffect(() => {
    const box = scroller.current;
    if (box && stick.current) box.scrollTop = box.scrollHeight;
  }, [events, run.state]);

  return (
    <article className="run-feed" aria-label={`Run: ${run.title}`}>
      <header className="feed-header">
        <div>
          <h2>{run.title}</h2>
          <p className="muted">
            {agent} · {modelName(run.runtime as Runtime, run.model ?? "")} · {stateLabel(run.state)}
            {run.branch ? ` · ${run.branch}` : ""}
          </p>
        </div>
        {active && (
          <button
            type="button"
            className="button"
            onClick={() => cancelRun(run.id).catch((cause) => onError(message(cause)))}
          >
            Cancelar
          </button>
        )}
      </header>
      <div
        ref={scroller}
        className="feed-scroll"
        onScroll={(event) => {
          const box = event.currentTarget;
          stick.current = box.scrollHeight - box.scrollTop - box.clientHeight < 80;
        }}
      >
        {feed.length === 0 && <p className="feed-status">Aguardando o agente…</p>}
        {feed.map((item) => (
          <FeedEntry key={item.seq} item={item} agent={agent} />
        ))}
        {active && (
          <p className="feed-working" role="status">
            <span className="pulse" aria-hidden="true" /> {agent} trabalhando…
          </p>
        )}
        {!active && (
          <footer className="feed-end">
            {run.summary && <p>{run.summary}</p>}
            <ProofLine proof={run.proof} />
            {run.branch && (
              <p className="muted">
                Branch <code>{run.branch}</code>: revise e teste antes do merge.
              </p>
            )}
            {canTryNextModel(run) && (
              <div className="run-next-model">
                <p>
                  As verificações não passaram com {run.lastModel}. Um ciclo com {run.nextModel}{" "}
                  continua a partir das alterações já feitas.
                </p>
                <button
                  type="button"
                  className="button"
                  onClick={() => continueRun(run.id).catch((cause) => onError(message(cause)))}
                >
                  Tentar com {run.nextModel}
                </button>
              </div>
            )}
            {proposed && (
              <StepsProposal
                key={run.id}
                run={run}
                proposed={proposed}
                onCreated={onStepsCreated}
                onError={onError}
              />
            )}
          </footer>
        )}
      </div>
    </article>
  );
}
