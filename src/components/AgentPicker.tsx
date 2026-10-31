import { useId } from "react";
import {
  type AgentChoice,
  agentOf,
  agents,
  choiceSummary,
  effortsOf,
  validEffort,
  type ReviewWhen,
  reviewWhens,
} from "../lib/agents";
import type { Runtime } from "../lib/roles";
import "../styles/agent-picker.css";

interface Props {
  choice: AgentChoice;
  onChange: (choice: AgentChoice) => void;
  /** Esconde o bloco de revisão (ex.: quando a Ação só lê). */
  compact?: boolean;
  /** Já aberto ao montar: no detalhe da tarefa, escolher quem faz o quê é parte do trabalho. */
  defaultOpen?: boolean;
}

interface AgentFieldsProps {
  id: string;
  runtime: Runtime;
  model: string;
  effort: string;
  /** Modelo "automático" (vazio) permitido: só o revisor Claude. */
  auto?: boolean;
  onChange: (runtime: Runtime, model: string) => void;
  onEffort: (effort: string) => void;
}

/** Agente + modelo: lista para o Claude Code; lista com id livre para o Codex. */
function AgentFields({ id, runtime, model, effort, auto, onChange, onEffort }: AgentFieldsProps) {
  const agent = agentOf(runtime);
  return (
    <>
      <label htmlFor={`${id}-agent`}>
        Agente
        <select
          id={`${id}-agent`}
          value={runtime}
          onChange={(event) => {
            const next = agentOf(event.target.value as Runtime);
            if (next)
              onChange(next.runtime, auto && next.runtime === "claude" ? "" : next.defaultModel);
          }}
        >
          {agents.map((item) => (
            <option key={item.runtime} value={item.runtime}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
      <label htmlFor={`${id}-model`}>
        Modelo
        {agent?.customModel ? (
          <>
            <input
              id={`${id}-model`}
              list={`${id}-models`}
              value={model === agent.defaultModel ? "" : model}
              placeholder="Padrão do Codex"
              maxLength={64}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) =>
                onChange(runtime, event.target.value.trim() || agent.defaultModel)
              }
            />
            <datalist id={`${id}-models`}>
              {agent.models.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                </option>
              ))}
            </datalist>
          </>
        ) : (
          <select
            id={`${id}-model`}
            value={model}
            onChange={(event) => onChange(runtime, event.target.value)}
          >
            {auto && <option value="">Automático</option>}
            {agent?.models.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        )}
      </label>
      <label htmlFor={`${id}-effort`}>
        Esforço
        <select
          id={`${id}-effort`}
          value={effort}
          onChange={(event) => onEffort(event.target.value)}
        >
          <option value="">{auto ? "Automático" : "Padrão"}</option>
          {effortsOf(runtime).map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
    </>
  );
}

/**
 * Configuração da run num bloco recolhível: o resumo mostra quem implementa e quem
 * revisa; aberto, deixa escolher agente e modelo de cada um e quando revisar.
 */
export function AgentPicker({ choice, onChange, compact, defaultOpen = false }: Props) {
  const id = useId();
  return (
    <details className="agent-picker" open={defaultOpen || undefined}>
      <summary>
        {compact ? choiceSummary({ ...choice, reviewWhen: "never" }) : choiceSummary(choice)}
      </summary>
      <fieldset>
        <legend>Implementa</legend>
        <AgentFields
          id={`${id}-impl`}
          runtime={choice.runtime}
          model={choice.model}
          effort={choice.effort}
          onChange={(runtime, model) =>
            onChange({ ...choice, runtime, model, effort: validEffort(runtime, choice.effort) })
          }
          onEffort={(effort) => onChange({ ...choice, effort })}
        />
        <p className="picker-help">Mais esforço pensa mais, custa mais e demora mais.</p>
      </fieldset>
      {!compact && (
        <fieldset>
          <legend>Revisa</legend>
          <label htmlFor={`${id}-when`}>
            Quando
            <select
              id={`${id}-when`}
              value={choice.reviewWhen}
              onChange={(event) =>
                onChange({ ...choice, reviewWhen: event.target.value as ReviewWhen })
              }
            >
              {reviewWhens.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                </option>
              ))}
            </select>
          </label>
          {choice.reviewWhen !== "never" && (
            <AgentFields
              id={`${id}-rev`}
              runtime={choice.reviewer}
              model={choice.reviewerModel}
              effort={choice.reviewerEffort}
              auto
              onChange={(reviewer, reviewerModel) =>
                onChange({
                  ...choice,
                  reviewer,
                  reviewerModel,
                  reviewerEffort: validEffort(reviewer, choice.reviewerEffort),
                })
              }
              onEffort={(reviewerEffort) => onChange({ ...choice, reviewerEffort })}
            />
          )}
          <p className="picker-help">
            {choice.reviewWhen === "auto"
              ? "Proporcional (recomendado): pula mudança pequena e sem risco, como só documentação. Em Automático, o modelo e o esforço seguem o tamanho da mudança: menores nas pequenas, maiores nas grandes ou delicadas."
              : choice.reviewWhen === "always"
                ? "Revisa toda run, mesmo uma linha de texto."
                : "Não revisa sozinho; use Revisar agora quando quiser."}
          </p>
        </fieldset>
      )}
    </details>
  );
}
