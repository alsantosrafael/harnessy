import { type ReactNode, useCallback, useEffect, useState } from "react";
import { Github, RefreshCw, ShieldCheck, Sparkles, Terminal } from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import {
  checkConnections,
  type ConnectionState,
  type ConnectionStatus,
  type Provider,
} from "../lib/desktop";

const providers: {
  provider: Provider;
  title: string;
  description: string;
  icon: ReactNode;
}[] = [
  {
    provider: "github",
    title: "GitHub",
    description: "Seus repositórios, boards e pull requests, no mesmo fluxo.",
    icon: <Github aria-hidden="true" />,
  },
  {
    provider: "codex",
    title: "Codex",
    description: "Implementação, revisão e testes em sessões independentes.",
    icon: <Terminal aria-hidden="true" />,
  },
  {
    provider: "claude",
    title: "Claude Code",
    description: "Runtime alternativo para papéis de agente, com a sessão da sua conta.",
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
    <section className="page connections-page">
      <p className="eyebrow">O COMEÇO DE CADA ENTREGA</p>
      <h1>Conecte suas ferramentas.</h1>
      <p className="lead">
        Uma interface. Suas contas. O Harnessy reutiliza a sessão das CLIs oficiais e nunca lê nem
        guarda suas credenciais.
      </p>
      <div className="connections-toolbar">
        <span role="status">
          {!desktop
            ? "A verificação só funciona no aplicativo desktop."
            : checking
              ? "Verificando sessões das CLIs…"
              : statuses
                ? `${connected} de ${providers.length} conexões ativas.`
                : ""}
        </span>
        <button className="button" onClick={refresh} disabled={!desktop || checking}>
          <RefreshCw size={15} aria-hidden="true" />
          Verificar novamente
        </button>
      </div>
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
      <div className="notice">
        <ShieldCheck size={19} aria-hidden="true" />
        <div>
          <strong>Autenticação pelo terminal, não pelo aplicativo</strong>
          <p>
            Para conectar ou renovar uma sessão, execute o comando indicado no seu terminal e depois
            use “Verificar novamente”. Tokens ficam no keychain ou nos arquivos de cada CLI; a
            interface recebe apenas o estado da conexão.
          </p>
        </div>
      </div>
    </section>
  );
}

function Connection({
  icon,
  title,
  description,
  status,
  phase,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  status: ConnectionStatus | null;
  phase: "web" | "checking" | "ready";
}) {
  const state: ConnectionState | "checking" | "web" =
    phase === "web" ? "web" : (status?.state ?? "checking");
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
        <span className="badge">
          <span className={`status-dot status-${state}`} aria-hidden="true" />
          {stateLabel[state]}
        </span>
      </div>
      <h2>{title}</h2>
      <p>{description}</p>
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
      <span className="connection-footnote">
        {state === "connected"
          ? "Pronta para uso quando os agentes forem habilitados."
          : state === "web"
            ? "Disponível no aplicativo desktop"
            : "Depois de autenticar, verifique novamente."}
      </span>
    </article>
  );
}
