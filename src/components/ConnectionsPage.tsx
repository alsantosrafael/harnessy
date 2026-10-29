import type { ReactNode } from "react";
import { ArrowRight, Github, ShieldCheck, Terminal } from "lucide-react";

export function ConnectionsPage() {
  return (
    <section className="page connections-page">
      <p className="eyebrow">O COMEÇO DE CADA ENTREGA</p>
      <h1>Conecte suas ferramentas.</h1>
      <p className="lead">Uma interface. Suas contas. Credenciais fora das conversas.</p>
      <div className="connections-grid">
        <Connection
          icon={<Github aria-hidden="true" />}
          title="GitHub"
          description="Seus repositórios, boards e pull requests, no mesmo fluxo."
          state="Não conectado"
          button="Conectar ao GitHub"
        />
        <Connection
          icon={<Terminal aria-hidden="true" />}
          title="Codex"
          description="Implementação, revisão e testes em sessões independentes."
          state="Não verificado"
          button="Conectar ao Codex"
        />
      </div>
      <div className="notice">
        <ShieldCheck size={19} aria-hidden="true" />
        <div>
          <strong>Conexões ainda não habilitadas</strong>
          <p>
            OAuth, armazenamento seguro e isolamento serão validados antes de liberar agentes.
            Nenhuma credencial é solicitada nesta versão.
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
  state,
  button,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  state: string;
  button: string;
}) {
  return (
    <article className="connection-card">
      <div className="connection-top">
        <span className="connection-icon">{icon}</span>
        <span className="badge">{state}</span>
      </div>
      <h2>{title}</h2>
      <p>{description}</p>
      <button className="button" disabled>
        {button}
        <ArrowRight size={15} aria-hidden="true" />
      </button>
      <span className="connection-footnote">Disponível em uma próxima etapa</span>
    </article>
  );
}
