import { mergeBlocker, stageOf } from "./delivery";
import type { Delivery, LocalTask, PreviewInfo, Run } from "./desktop";
import { isLive } from "./preview";
import { isActive, reviewable } from "./runs";

export type QuickId =
  | "implement"
  | "review"
  | "preview"
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
  /** Ambiente de teste da run que carrega o trabalho, se já foi ligado. */
  preview?: PreviewInfo | null;
}

const DONE_REASON = "Tarefa concluída. Reabra para continuar.";
const LOCKED_WHEN_DONE: ReadonlySet<QuickId> = new Set([
  "implement",
  "review",
  "preview",
  "publish",
  "merge",
]);

/** Comandos rápidos da tarefa, com o motivo de cada indisponibilidade. */
export function quickStates({
  task,
  lastRun,
  canStart,
  starting,
  dirty,
  workRun = null,
  delivery = null,
  preview = null,
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
      id: "preview",
      label: isLive(preview) ? "Parar ambiente" : "Rodar localmente",
      reason: !workRun
        ? "Ainda não há run concluída com alterações."
        : workRun.state !== "completed"
          ? "A run não terminou com sucesso."
          : busy
            ? "Há uma run em andamento."
            : null,
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
  // Tarefa concluída é só leitura: o que muda o trabalho fica travado até reabrir.
  if (task.status === "done") {
    return states.map((state) =>
      LOCKED_WHEN_DONE.has(state.id) ? { ...state, reason: DONE_REASON } : state,
    );
  }
  return states;
}

/** A ação que o laço pede agora; é o botão principal da tarefa. */
export interface NextAction {
  id: QuickId;
  label: string;
  /** Por que esta é a próxima: uma frase que o humano confere de relance. */
  why: string;
}

/**
 * Próximo passo da tarefa, deduzido do estágio de entrega. `previewSeen` diz que o
 * ambiente de teste desta run já foi ligado, e então o passo deixa de ser "testar".
 * Devolve `null` quando o certo é esperar (o agente trabalha) ou não há o que fazer.
 */
export function nextAction(input: Input & { previewSeen?: boolean }): NextAction | null {
  const { task, lastRun, workRun = null, delivery = null, previewSeen = false } = input;
  if (task.status === "done") return null;
  if (input.starting || (lastRun !== null && isActive(lastRun))) return null;
  const states = quickStates(input);
  const open = (id: QuickId) => states.some((state) => state.id === id && state.reason === null);
  const pick = (id: QuickId, label: string, why: string): NextAction | null =>
    open(id) ? { id, label, why } : null;

  const stage = stageOf(workRun, delivery);
  if (!stage) {
    if (!lastRun) return pick("implement", "Implementar", "A tarefa ainda não tem nenhuma run.");
    const again = lastRun.state === "failed" || lastRun.state === "no_changes";
    return pick(
      "implement",
      again ? "Tentar de novo" : "Implementar",
      again ? "A última run não entregou alterações." : "Ainda não há alterações para entregar.",
    );
  }
  switch (stage.id) {
    case "local":
      return previewSeen
        ? pick("publish", "Publicar PR", "Você já testou localmente: abra o PR em draft.")
        : pick("preview", "Rodar localmente", "Veja a mudança funcionando antes de publicar.");
    case "draft":
    case "open":
      return pick("review", "Revisar", "O PR ainda não tem uma revisão aprovada neste commit.");
    case "checks-running":
      return pick("publish", "Atualizar PR", "Os checks ainda estão rodando.");
    case "checks-failing":
    case "conflict":
    case "changes-requested":
      return pick("conversation", "Pedir ajuste", stage.hint);
    case "ready":
      return pick("merge", "Fazer merge", "Checks sem falha e revisão aprovada neste commit.");
    case "closed":
      return pick("publish", "Publicar de novo", "O PR foi fechado sem merge.");
    default:
      return null;
  }
}
