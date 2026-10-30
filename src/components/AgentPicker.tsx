import { useId } from "react";
import { type AgentChoice, agentOf, agents } from "../lib/agents";
import type { Runtime } from "../lib/roles";
import "../styles/agent-picker.css";

interface Props {
  choice: AgentChoice;
  onChange: (choice: AgentChoice) => void;
}

/** Escolhe o agente e o modelo do Implementador. Agente sem adaptador aparece desabilitado, com o motivo. */
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
              onChange({ runtime: next.runtime, model: next.defaultModel });
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
    </div>
  );
}
