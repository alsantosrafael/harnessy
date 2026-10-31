import { mergeBlocker } from "./delivery";
import type { Delivery, LocalTask, Run } from "./desktop";
import { isActive, reviewable } from "./runs";

export type QuickId =
  | "implement"
  | "review"
  | "publish"
  | "merge"
  | "conversation"
  | "branch"
  | "finish"
  | "reopen";

export interface QuickState {
  id: QuickId;
  label: string;
  /** Motivo de estar indisponível, dito em texto; `null` quando dá para usar. */
  reason: string | null;
}

interface Input {
  task: LocalTask;
  /** Run mais recente da tarefa, se houver. */
  lastRun: Run | null;
  /** Agente e git disponíveis e nenhuma run ativa no workspace. */
  canStart: boolean;
  starting: boolean;
  /** A tarefa tem edições não salvas: o agente leria o texto antigo. */
  dirty: boolean;
  /** Run que carrega o trabalho (a mais recente com branch) e o PR dela, se já existir. */
  workRun?: Run | null;
  delivery?: Delivery | null;
}

/** Comandos rápidos da tarefa, com o motivo de cada indisponibilidade. */
export function quickStates({
  task,
  lastRun,
  canStart,
  starting,
  dirty,
  workRun = null,
  delivery = null,
}: Input): QuickState[] {
  const busy = starting || (lastRun !== null && isActive(lastRun));
  const states: QuickState[] = [
    {
      id: "implement",
      label: "Implementar",
      reason: dirty
        ? "Salve a tarefa antes."
        : busy
          ? "Há uma run em andamento."
          : canStart
            ? null
            : "Agente ou git indisponível, ou outra run ativa.",
    },
    {
      id: "review",
      label: lastRun?.review ? "Revisar de novo" : "Revisar última run",
      reason: !lastRun
        ? "Ainda não há run."
        : !reviewable(lastRun)
          ? "A última run não terminou com alterações na branch."
          : canStart
            ? null
            : "Espere a run em andamento terminar.",
    },
    {
      id: "publish",
      label: delivery ? "Atualizar PR" : "Publicar PR",
      reason: !workRun
        ? "Ainda não há run concluída com alterações."
        : workRun.state !== "completed"
          ? "A run não terminou com sucesso."
          : busy
            ? "Há uma run em andamento."
            : delivery?.state === "merged"
              ? "O PR já foi mesclado."
              : null,
    },
    {
      id: "merge",
      label: "Fazer merge",
      reason: busy ? "Há uma run em andamento." : mergeBlocker(workRun, delivery),
    },
    {
      id: "conversation",
      label: "Abrir conversa",
      reason: lastRun ? null : "Ainda não há run.",
    },
    {
      id: "branch",
      label: "Copiar branch",
      reason: lastRun?.branch ? null : "A última run não criou branch.",
    },
  ];
  states.push(
    task.status === "done"
      ? { id: "reopen", label: "Reabrir tarefa", reason: null }
      : { id: "finish", label: "Concluir tarefa", reason: null },
  );
  return states;
}
