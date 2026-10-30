import { type ActionInfo, IMPLEMENT_ACTION } from "./desktop";

/** Ações que aparecem em "Mais ações": as que agem sobre a tarefa, exceto a principal
 * (que o botão "Implementar" dispara). Ações sobre run (revisor) rodam sozinhas. */
export const extraActions = (actions: readonly ActionInfo[]): ActionInfo[] =>
  actions.filter((action) => action.id !== IMPLEMENT_ACTION && action.alvo !== "run");

/** Motivo para a Ação não poder ser iniciada, ou `null` quando pode. */
export const unavailableReason = (action: ActionInfo): string | null => action.erro ?? null;

/** Rótulo do nível de acesso, para a Ação na lista. */
export const accessLabel = (acesso: ActionInfo["acesso"]): string => {
  if (acesso === "leitura") return "Só leitura";
  if (acesso === "escrita") return "Altera o repositório";
  return "";
};

/** Variáveis que o pedido de uma Ação sobre tarefa pode usar. */
export const TASK_VARIABLES = ["tarefa", "descricao", "criterios", "arquivos_provaveis"] as const;

/** Id sugerido a partir do nome: minúsculas, sem acento, hífen no lugar de espaço, até 40. */
export function suggestId(nome: string): string {
  return nome
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40)
    .replace(/-+$/g, "");
}

/** Rótulo da origem, para a Ação na lista. */
export const originLabel = (origem: ActionInfo["origem"]): string =>
  origem === "repositorio" ? "Do repositório" : origem === "usuario" ? "Sua" : "Embutida";
