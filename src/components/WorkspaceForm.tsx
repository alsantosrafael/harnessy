import { useState } from "react";
import { createWorkspace, type Workspace } from "../lib/desktop";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

interface Props {
  onCreated: (workspace: Workspace) => void;
}

/** Vincula uma pasta local a um workspace. Não importa nem modifica arquivos. */
export function WorkspaceForm({ onCreated }: Props) {
  const [name, setName] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  return (
    <form
      className="local-form workspace-form"
      onSubmit={async (event) => {
        event.preventDefault();
        setBusy(true);
        setError("");
        try {
          onCreated(await createWorkspace(name, folder));
        } catch (cause) {
          setError(message(cause));
        } finally {
          setBusy(false);
        }
      }}
    >
      <label>
        Nome do workspace
        <input
          required
          maxLength={200}
          value={name}
          onChange={(event) => setName(event.target.value)}
          disabled={busy}
        />
      </label>
      <label>
        Pasta local
        <input
          required
          placeholder="/Users/seu-usuario/projetos/meu-projeto"
          value={folder}
          onChange={(event) => setFolder(event.target.value)}
          disabled={busy}
          aria-describedby="folder-hint"
        />
      </label>
      <p id="folder-hint">
        Informe um caminho absoluto existente. O vínculo não importa nem modifica arquivos.
      </p>
      {error && (
        <p className="local-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary" disabled={busy}>
        {busy ? "Salvando…" : "Criar workspace"}
      </button>
    </form>
  );
}
