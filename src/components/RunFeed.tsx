import { CircleCheck, Info, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import {
  cancelRun,
  continueRun,
  createSteps,
  listTasks,
  type LocalTask,
  type Refinement,
  type Run,
  type RunEvent,
  saveTask,
  type StepProposal,
} from "../lib/desktop";
import { type FeedItem, type ReviewPayload, buildFeed } from "../lib/feed";
import { type AgentChoice, agentOf, modelName, reviewerLabel } from "../lib/agents";
import type { Runtime } from "../lib/roles";
import {
  canTryNextModel,
  isActive,
  ledgerItems,
  type Decision,
  decisionsOf,
  MAX_REFINE_ROUNDS,
  proofLabel,
  type ProofTone,
  refineInstruction,
  refinementFrom,
  refineRounds,
  withDecisions,
  reviewable,
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

/** Rascunho do refinamento: responda às perguntas, refine de novo ou aplique à tarefa. */
function RefinementProposal({
  run,
  draft,
  earlier,
  canRefine,
  onRefine,
  onApplied,
  onError,
}: {
  run: Run;
  draft: Refinement;
  /** Decisões das rodadas anteriores, registradas na descrição ao aplicar. */
  earlier: Decision[];
  /** Ainda há rodada disponível (limite por conversa). */
  canRefine: boolean;
  onRefine: (taskId: number, instruction: string, decisions: Decision[]) => Promise<void>;
  onApplied: () => void;
  onError: (text: string) => void;
}) {
  const [answers, setAnswers] = useState(() => draft.questions.map((q) => q.recommended));
  const [description, setDescription] = useState(draft.description);
  const [criteria, setCriteria] = useState(draft.criteria);
  const [busy, setBusy] = useState(false);
  const [applied, setApplied] = useState(false);
  // Revisão da tarefa quando o rascunho apareceu: se ela mudou depois, salvar recusa em vez de sobrescrever.
  const [revision, setRevision] = useState<number | null>(null);
  useEffect(() => {
    let live = true;
    listTasks(run.workspaceId)
      .then((tasks) => {
        const found = tasks.find((task) => task.id === run.taskId);
        if (live && found) setRevision(found.revision);
      })
      .catch((cause) => onError(message(cause)));
    return () => {
      live = false;
    };
  }, [run.workspaceId, run.taskId, onError]);
  if (applied) return <p role="status">Descrição e critérios aplicados à tarefa.</p>;

  const decisions = [...earlier, ...decisionsOf(draft.questions, answers)];
  const apply = async () => {
    const current = (await listTasks(run.workspaceId)).find((task) => task.id === run.taskId);
    if (!current || revision === null) throw new Error("A tarefa não existe mais.");
    await saveTask({
      id: current.id,
      workspaceId: current.workspaceId,
      revision,
      title: current.title,
      description: withDecisions(description, decisions),
      criteria,
    });
    setApplied(true);
    onApplied();
  };
  const guard = (work: () => Promise<void>) => {
    setBusy(true);
    work()
      .catch((cause) => onError(message(cause)))
      .finally(() => setBusy(false));
  };
  return (
    <form
      className="local-form refinement-proposal"
      onSubmit={(event) => {
        event.preventDefault();
        guard(apply);
      }}
    >
      <h4>Refinamento proposto</h4>
      {draft.ready && (
        <p role="status">
          <CircleCheck aria-hidden="true" size={16} /> Pronta para implementar: nenhuma pergunta em
          aberto muda o rascunho.
        </p>
      )}
      {draft.questions.length > 0 && (
        <>
          <p>
            Responda ou ajuste. A resposta recomendada já vem preenchida: deixe como está para
            aceitá-la.
          </p>
          <ol>
            {draft.questions.map((item, at) => (
              <li key={item.question}>
                <label>
                  {item.question}
                  <textarea
                    rows={2}
                    value={answers[at] ?? ""}
                    disabled={busy}
                    onChange={(event) =>
                      setAnswers((current) =>
                        current.map((value, i) => (i === at ? event.target.value : value)),
                      )
                    }
                  />
                </label>
              </li>
            ))}
          </ol>
        </>
      )}
      <label>
        Descrição proposta
        <textarea
          rows={5}
          value={description}
          disabled={busy}
          onChange={(event) => setDescription(event.target.value)}
        />
      </label>
      <label>
        Critérios propostos
        <textarea
          rows={5}
          value={criteria}
          disabled={busy}
          onChange={(event) => setCriteria(event.target.value)}
        />
      </label>
      {decisions.length > 0 && (
        <details>
          <summary>{decisions.length} decisão(ões) serão registradas na descrição</summary>
          <ul>
            {decisions.map((item) => (
              <li key={item.question}>
                {item.question} → {item.answer}
              </li>
            ))}
          </ul>
        </details>
      )}
      <p className="muted">Aplicar substitui a descrição e os critérios atuais da tarefa.</p>
      {!canRefine && draft.questions.length > 0 && (
        <p className="muted">
          Limite de {MAX_REFINE_ROUNDS} rodadas atingido. Aplique e ajuste o texto à mão.
        </p>
      )}
      <div className="composer-actions">
        {canRefine && draft.questions.length > 0 && (
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() =>
              guard(() =>
                onRefine(
                  run.taskId,
                  refineInstruction(draft.questions, answers, description, criteria),
                  decisions,
                ),
              )
            }
          >
            Refinar de novo
          </button>
        )}
        <button
          className="button primary"
          disabled={busy || revision === null || !description.trim()}
        >
          {busy ? "Aplicando…" : "Aplicar à tarefa"}
        </button>
      </div>
    </form>
  );
}

interface Props {
  /** Runs da tarefa, da mais antiga à mais nova: a conversa inteira. */
  thread: Run[];
  eventsByRun: Record<number, RunEvent[]>;
  onError: (text: string) => void;
  /** Passos confirmados viraram tarefas. */
  onStepsCreated: (tasks: LocalTask[]) => void;
  /** Um refinamento foi aplicado à tarefa. */
  onTaskChanged: () => void;
  /** Nova rodada de refinamento com as respostas; rejeita se o núcleo recusar. */
  onRefine: (taskId: number, instruction: string) => Promise<void>;
  /** Revisor escolhido, mostrado no botão de revisar. */
  choice: AgentChoice;
  /** Nenhuma outra run em andamento: a revisão sob demanda pode começar. */
  canReview: boolean;
  /** Pede a revisão desta run; rejeita se o núcleo recusar. */
  onReview: (runId: number) => Promise<void>;
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
    case "user":
      return (
        <div className="feed-user">
          <span className="feed-who">Você</span>
          <p>{item.text}</p>
        </div>
      );
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

/** Conversa da tarefa: o que você pediu, o que o agente disse e fez em cada run, e as ações de agora. */
export function RunFeed({
  thread,
  eventsByRun,
  onError,
  onStepsCreated,
  onTaskChanged,
  onRefine,
  choice,
  canReview,
  onReview,
}: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  const run = thread[thread.length - 1];
  const events = eventsByRun[run.id] ?? [];
  const active = isActive(run);
  const proposed = run.state === "completed" ? stepsFrom(events) : null;
  const refinement = run.state === "completed" ? refinementFrom(events) : null;
  // Decisões já tomadas em rodadas anteriores desta conversa (só em memória; o limite é de 2 rodadas).
  const [earlier, setEarlier] = useState<{ taskId: number; decisions: Decision[] } | null>(null);
  const canRefine = refineRounds(thread, eventsByRun) < MAX_REFINE_ROUNDS;
  const agent = agentLabel(run.runtime);
  const eventTotal = thread.reduce((sum, item) => sum + (eventsByRun[item.id]?.length ?? 0), 0);
  // Revisão sob demanda: "revisando" vale até chegar o veredito ou o aviso de que não rodou.
  const [since, setSince] = useState<number | null>(null);
  const reviewing =
    since !== null &&
    !events.some(
      (event) =>
        event.seq > since &&
        (event.kind === "review" || event.text.startsWith("Revisão não rodou")),
    );

  // Acompanha o fim da conversa, a menos que a pessoa tenha rolado para ler algo acima.
  // biome-ignore lint/correctness/useExhaustiveDependencies: o total de eventos e o estado disparam o acompanhamento.
  useEffect(() => {
    const box = scroller.current;
    if (box && stick.current) box.scrollTop = box.scrollHeight;
  }, [eventTotal, run.state]);

  return (
    <article className="run-feed" aria-label={`Conversa: ${run.title}`}>
      <header className="feed-header">
        <div>
          <h2>{run.title}</h2>
          <p className="muted">
            {thread.length > 1 ? `${thread.length} runs · ` : ""}
            {agent} · {modelName(run.runtime as Runtime, run.model ?? "")} · {stateLabel(run.state)}
            {run.branch ? ` · ${run.branch}` : ""}
          </p>
          <p className="muted run-ledger">
            {ledgerItems(run).map((item, at) => (
              <span key={item.label}>
                {at > 0 && " · "}
                {item.label} {item.value}
              </span>
            ))}
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
        {thread.map((item) => {
          const last = item.id === run.id;
          const feed = buildFeed(eventsByRun[item.id] ?? []);
          return (
            <section key={item.id} className="feed-turn" aria-label={`Run #${item.id}`}>
              {thread.length > 1 && (
                <h3 className="feed-run-mark">
                  Run #{item.id} · {stateLabel(item.state)}
                </h3>
              )}
              {feed.length === 0 && last && <p className="feed-status">Aguardando o agente…</p>}
              {feed.map((entry) => (
                <FeedEntry key={entry.seq} item={entry} agent={agentLabel(item.runtime)} />
              ))}
              {!last && (
                <footer className="feed-end is-past">
                  {item.summary && <p>{item.summary}</p>}
                  <ProofLine proof={item.proof} />
                </footer>
              )}
            </section>
          );
        })}
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
            {reviewable(run) && (
              <div className="review-now">
                <button
                  type="button"
                  className="button"
                  disabled={!canReview || reviewing}
                  onClick={() => {
                    setSince(events.reduce((max, event) => Math.max(max, event.seq), 0));
                    onReview(run.id).catch((cause) => {
                      setSince(null);
                      onError(message(cause));
                    });
                  }}
                >
                  {reviewing ? "Revisando…" : run.review ? "Revisar de novo" : "Revisar agora"}
                </button>
                <small className="muted">
                  Revisor: {reviewerLabel(choice)}. Roda mesmo sem sinal de risco.
                </small>
              </div>
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
            {refinement && (
              <RefinementProposal
                key={run.id}
                run={run}
                draft={refinement}
                earlier={earlier?.taskId === run.taskId ? earlier.decisions : []}
                canRefine={canRefine}
                onRefine={async (taskId, instruction, decisions) => {
                  await onRefine(taskId, instruction);
                  setEarlier({ taskId, decisions });
                }}
                onApplied={() => {
                  setEarlier(null);
                  onTaskChanged();
                }}
                onError={onError}
              />
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
