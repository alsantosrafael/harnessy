import { type ReactNode, useCallback, useEffect, useState } from "react";
import { Github, RefreshCw, Sparkles, Terminal } from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import {
  checkConnections,
  type ConnectionState,
  type ConnectionStatus,
  type Provider,
} from "../lib/desktop";
import { rolesFor } from "../lib/roles";

const providers: {
  provider: Provider;
  title: string;
  icon: ReactNode;
}[] = [
  {
    provider: "github",
    title: "GitHub",
    icon: <Github aria-hidden="true" />,
  },
  {
    provider: "codex",
    title: "Codex",
    icon: <Terminal aria-hidden="true" />,
  },
  {
    provider: "claude",
    title: "Claude Code",
    icon: <Sparkles aria-hidden="true" />,
  },
];

const stateLabel: Record<ConnectionState | "checking" | "web", string> = {
  connected: "Conectado",
  expired: "Sessão expirada",
  unavailable: "Indisponível",
  unknown: "Não verificado",
  checking: "Verificando…",
  web: "Somente no desktop",
};

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function ConnectionsPage() {
  const desktop = isTauri();
  const [statuses, setStatuses] = useState<ConnectionStatus[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setChecking(true);
    setError("");
    try {
      setStatuses(await checkConnections());
    } catch (cause) {
      setError(message(cause));
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    if (desktop) void refresh();
  }, [desktop, refresh]);

  const connected = statuses?.filter((status) => status.state === "connected").length ?? 0;

  return (
    <section className="page connections-page" aria-label="Conexões">
      <header className="page-header">
        <h1>Conexões</h1>
        <span className="muted" role="status">
          {!desktop
            ? "Verificação só no aplicativo desktop"
            : checking
              ? "Verificando…"
              : statuses
                ? `${connected} de ${providers.length} ativas`
                : ""}
        </span>
        <div className="page-actions">
          <button className="button" onClick={refresh} disabled={!desktop || checking}>
            <RefreshCw size={14} aria-hidden="true" />
            Verificar
          </button>
        </div>
      </header>
      {error && (
        <div className="local-error" role="alert">
          {error}
        </div>
      )}
      <div className="connections-grid">
        {providers.map((item) => (
          <Connection
            key={item.provider}
            {...item}
            status={statuses?.find((status) => status.provider === item.provider) ?? null}
            phase={!desktop ? "web" : checking && !statuses ? "checking" : "ready"}
          />
        ))}
      </div>
    </section>
  );
}

function Connection({
  provider,
  icon,
  title,
  status,
  phase,
}: {
  provider: Provider;
  icon: ReactNode;
  title: string;
  status: ConnectionStatus | null;
  phase: "web" | "checking" | "ready";
}) {
  const state: ConnectionState | "checking" | "web" =
    phase === "web" ? "web" : (status?.state ?? "checking");
  const roleNames =
    provider === "github"
      ? null
      : rolesFor(provider)
          .map((item) => item.label)
          .join(", ");
  const command =
    status && status.state === "unavailable"
      ? status.installHint
      : status && status.state !== "connected"
        ? status.loginCommand
        : null;
  return (
    <article className="connection-card" data-state={state}>
      <div className="connection-top">
        <span className="connection-icon">{icon}</span>
        <h2>{title}</h2>
        <span className="badge">
          <span className={`status-dot status-${state}`} aria-hidden="true" />
          {stateLabel[state]}
        </span>
      </div>
      {roleNames && <p className="connection-roles">{roleNames}</p>}
      <dl className="connection-facts">
        <dt>Estado</dt>
        <dd>
          {phase === "web"
            ? "Abra o aplicativo desktop para verificar esta conexão."
            : (status?.detail ?? "Aguardando resposta da CLI…")}
        </dd>
        {status?.account && (
          <>
            <dt>Conta</dt>
            <dd>{status.account}</dd>
          </>
        )}
        {status?.binary && (
          <>
            <dt>CLI</dt>
            <dd>{status.binary}</dd>
          </>
        )}
      </dl>
      {command && (
        <div className="connection-command">
          <span>
            {status?.state === "unavailable" ? "Instale a CLI:" : "No terminal, execute:"}
          </span>
          <code>{command}</code>
        </div>
      )}
    </article>
  );
}
