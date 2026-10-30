import { useId } from "react";
import { type AgentChoice, type Reviewer, agentOf, agents, reviewers } from "../lib/agents";
import type { Runtime } from "../lib/roles";
import "../styles/agent-picker.css";

interface Props {
  choice: AgentChoice;
  onChange: (choice: AgentChoice) => void;
}

/** Escolhe o agente, o modelo e o revisor. Agente sem adaptador aparece desabilitado, com o motivo. */
export function AgentPicker({ choice, onChange }: Props) {
  const id = useId();
  const agent = agentOf(choice.runtime);
  return (
    <div className="agent-picker">
      <label htmlFor={`${id}-agent`}>
        Agente
        <select
          id={`${id}-agent`}
          value={choice.runtime}
          onChange={(event) => {
            const next = agentOf(event.target.value as Runtime);
            if (next && !next.unavailable) {
              onChange({ ...choice, runtime: next.runtime, model: next.defaultModel });
            }
          }}
        >
          {agents.map((item) => (
            <option key={item.runtime} value={item.runtime} disabled={Boolean(item.unavailable)}>
              {item.unavailable ? `${item.label} (${item.unavailable})` : item.label}
            </option>
          ))}
        </select>
      </label>
      <label htmlFor={`${id}-model`}>
        Modelo
        <select
          id={`${id}-model`}
          value={choice.model}
          onChange={(event) => onChange({ ...choice, model: event.target.value })}
        >
          {agent?.models.map((model) => (
            <option key={model.id} value={model.id}>
              {model.label}
            </option>
          ))}
        </select>
      </label>
      <label htmlFor={`${id}-reviewer`}>
        Revisor
        <select
          id={`${id}-reviewer`}
          value={choice.reviewer}
          onChange={(event) => onChange({ ...choice, reviewer: event.target.value as Reviewer })}
        >
          {reviewers.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
