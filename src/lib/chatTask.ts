const TITLE_MAX = 100;

/**
 * Transforma o rascunho do chat em tarefa: a primeira linha com texto vira o título
 * e o restante vira a descrição. Título longo é cortado e o texto inteiro vai para a
 * descrição, para não perder nada.
 */
export function splitDraft(text: string): { title: string; description: string } {
  const lines = text.trim().split("\n");
  const first = lines[0].trim();
  const rest = lines.slice(1).join("\n").trim();
  if (first.length <= TITLE_MAX) return { title: first, description: rest };
  return { title: `${first.slice(0, TITLE_MAX - 1).trimEnd()}…`, description: text.trim() };
}
