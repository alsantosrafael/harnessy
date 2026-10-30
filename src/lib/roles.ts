import type { Provider } from "./desktop";

/** Papéis de agente definidos na SPEC (seção "Papéis"). */
export type Role = "refinador" | "implementador" | "revisor" | "tester";

export type Runtime = Extract<Provider, "codex" | "claude">;

export interface RoleDefinition {
  role: Role;
  label: string;
  runtime: Runtime;
  delivers: string;
}

/**
 * Atribuição fixa de runtime por papel, decidida em 2026-10-29:
 * Claude Code implementa; Codex revisa e testa. A separação de runtimes
 * reforça a independência da revisão. Não há troca pela interface nesta versão.
 */
export const roles: readonly RoleDefinition[] = [
  {
    role: "refinador",
    label: "Refinador",
    runtime: "claude",
    delivers: "Objetivo, escopo e critérios de aceite.",
  },
  {
    role: "implementador",
    label: "Implementador",
    runtime: "claude",
    delivers: "Alterações, checks e resumo.",
  },
  {
    role: "revisor",
    label: "Revisor",
    runtime: "codex",
    delivers: "Achados fundamentados sobre o diff.",
  },
  {
    role: "tester",
    label: "Tester",
    runtime: "codex",
    delivers: "Cenários funcionais e evidências.",
  },
];

export const runtimeLabel: Record<Runtime, string> = {
  claude: "Claude Code",
  codex: "Codex",
};

/** Papéis atendidos por um runtime, na ordem do fluxo composto. */
export const rolesFor = (runtime: Runtime) => roles.filter((item) => item.runtime === runtime);
