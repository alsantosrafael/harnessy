import { Check, FolderGit2, GitBranch } from 'lucide-react';

export function ContextPanel() {
  return (
    <aside className="context-panel" id="context-panel" aria-label="Contexto do trabalho">
      <p className="section-label">CONTEXTO</p>
      <div className="context-heading">
        <span className="context-icon" aria-hidden="true"><GitBranch size={18} /></span>
        <h3>Tudo conectado.<br />Nada perdido.</h3>
      </div>
      <p>Cards, código e evidências do trabalho aparecem aqui, sem tirar você da conversa.</p>
      <div className="context-section">
        <span>REPOSITÓRIOS</span>
        <p><FolderGit2 size={14} aria-hidden="true" />Nenhum vinculado</p>
      </div>
      <div className="context-section">
        <span>AGENTES</span>
        {['Implementador', 'Revisor', 'Tester'].map(role => (
          <div className="agent-line" key={role}>
            <span className="agent-avatar" aria-hidden="true">{role[0]}</span>{role}<span className="idle-dot" aria-hidden="true" />
          </div>
        ))}
        <p className="context-caption">Papéis planejados · nenhuma sessão ativa</p>
      </div>
      <div className="context-tip"><Check size={14} aria-hidden="true" /><span>O merge sempre passa por você.</span></div>
    </aside>
  );
}
