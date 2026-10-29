import { ArrowRight, ChevronRight, Github, Inbox } from "lucide-react";

const columns = ["A fazer", "Em andamento", "Em revisão", "Concluído"];

export function BoardPanel({ hidden, onConnect }: { hidden: boolean; onConnect: () => void }) {
  return (
    <section id="board-panel" role="tabpanel" aria-labelledby="board-tab" hidden={hidden}>
      <div className="board-title">
        <h2>Tarefas do projeto</h2>
        <span className="badge">Aguardando conexão</span>
      </div>
      <div className="board-columns" aria-hidden="true">
        {columns.map((column, index) => (
          <div key={column}>
            <span className={`column-dot dot-${index}`} />
            <span>{column}</span>
            <span className="count">0</span>
          </div>
        ))}
      </div>
      <div className="board-empty">
        <span className="empty-art" aria-hidden="true">
          <Inbox size={30} strokeWidth={1.3} />
        </span>
        <h2>Seu próximo capítulo está em branco.</h2>
        <p>
          Escolha um projeto do GitHub para trazer seus cards.
          <br />O trabalho começa quando você decidir.
        </p>
        <button className="button" onClick={onConnect}>
          <Github size={16} aria-hidden="true" />
          Configurar GitHub
          <ArrowRight size={14} aria-hidden="true" />
        </button>
        <span className="empty-caption">Nenhum card fictício. Nenhum agente em execução.</span>
      </div>
      <div className="flow-strip">
        <Flow number="01" title="Defina" text="Uma intenção clara." />
        <ChevronRight size={14} aria-hidden="true" />
        <Flow number="02" title="Acompanhe" text="Papéis independentes." />
        <ChevronRight size={14} aria-hidden="true" />
        <Flow number="03" title="Valide" text="Evidências antes do merge." />
      </div>
    </section>
  );
}

function Flow({ number, title, text }: { number: string; title: string; text: string }) {
  return (
    <div>
      <span>{number}</span>
      <div>
        <strong>{title}</strong>
        <p>{text}</p>
      </div>
    </div>
  );
}
