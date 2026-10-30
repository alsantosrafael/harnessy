import { useEffect, useState } from "react";
import {
  createWorkspace,
  listWorkspaces,
  listTasks,
  saveTask,
  type Workspace,
  type LocalTask,
} from "../lib/desktop";
import "../styles/local-work.css";

const empty = { title: "", description: "", criteria: "" };
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function LocalWorkPanel({ hidden }: { hidden: boolean }) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<number | null>(null);
  const [tasks, setTasks] = useState<LocalTask[]>([]);
  const [selected, setSelected] = useState<LocalTask | null>(null);
  const [form, setForm] = useState(empty);
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const [folder, setFolder] = useState("");
  const [creating, setCreating] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [reload, setReload] = useState(0);
  const dirty =
    form.title !== (selected?.title ?? "") ||
    form.description !== (selected?.description ?? "") ||
    form.criteria !== (selected?.criteria ?? "");

  // biome-ignore lint/correctness/useExhaustiveDependencies: `reload` é o gatilho de "Recarregar dados".
  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError("");
    listWorkspaces()
      .then((rows) => {
        if (!alive) return;
        setWorkspaces(rows);
        setWorkspaceId((current) => current ?? rows[0]?.id ?? null);
      })
      .catch((cause) => {
        if (alive) setError(message(cause));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [reload]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `reload` é o gatilho de "Recarregar dados".
  useEffect(() => {
    if (workspaceId === null) return;
    let alive = true;
    setLoading(true);
    setTasks([]);
    listTasks(workspaceId)
      .then((rows) => {
        if (alive) setTasks(rows);
      })
      .catch((cause) => {
        if (alive) setError(message(cause));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [workspaceId, reload]);

  useEffect(() => {
    if (!dirty) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  function canLeave() {
    return !dirty || window.confirm("Descartar as alterações não salvas desta tarefa?");
  }
  function openTask(task: LocalTask | null) {
    if (!canLeave()) return;
    setSelected(task);
    setForm(
      task ? { title: task.title, description: task.description, criteria: task.criteria } : empty,
    );
    setEditing(true);
    setError("");
    setNotice("");
    requestAnimationFrame(() => document.getElementById("task-title")?.focus());
  }

  return (
    <section className="local-work" hidden={hidden} aria-label="Workspaces e tarefas locais">
      <div className="local-heading">
        <div>
          <p className="eyebrow">NESTE DISPOSITIVO</p>
          <h2>Seu trabalho, organizado.</h2>
          <p>Prepare tarefas locais com objetivos e critérios de aceite.</p>
        </div>
        <button className="button" onClick={() => setCreating(!creating)} aria-expanded={creating}>
          Novo workspace
        </button>
      </div>
      {error && (
        <div className="local-error" role="alert">
          {error}{" "}
          <button
            className="button"
            disabled={busy || loading}
            onClick={() => setReload(reload + 1)}
          >
            Recarregar dados
          </button>
        </div>
      )}
      {creating && (
        <form
          className="local-form workspace-form"
          onSubmit={async (event) => {
            event.preventDefault();
            setBusy(true);
            setError("");
            try {
              const workspace = await createWorkspace(name, folder);
              setWorkspaces((rows) => [...rows, workspace]);
              // Keep the current task editor intact when creating another workspace.
              if (workspaceId === null) setWorkspaceId(workspace.id);
              setCreating(false);
              setName("");
              setFolder("");
              setNotice("Workspace criado e salvo neste dispositivo.");
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
          <button className="button primary" disabled={busy}>
            {busy ? "Salvando…" : "Criar workspace"}
          </button>
        </form>
      )}
      <p role="status">{notice || (loading ? "Carregando dados locais…" : "")}</p>
      {!loading && workspaces.length === 0 && !creating && (
        <div className="local-empty">
          <h3>Comece pelo seu projeto.</h3>
          <p>Crie um workspace para guardar tarefas. GitHub e agentes não são necessários.</p>
          <button className="button primary" onClick={() => setCreating(true)}>
            Criar primeiro workspace
          </button>
        </div>
      )}
      {workspaces.length > 0 && (
        <>
          <label className="workspace-picker">
            Workspace
            <select
              value={workspaceId ?? ""}
              disabled={busy}
              onChange={(event) => {
                if (!canLeave()) return;
                setWorkspaceId(Number(event.target.value));
                setSelected(null);
                setForm(empty);
                setEditing(false);
                setError("");
                setNotice("");
              }}
            >
              {workspaces.map((workspace) => (
                <option key={workspace.id} value={workspace.id}>
                  {workspace.name}
                </option>
              ))}
            </select>
          </label>
          <p className="local-folder">
            {workspaces.find((workspace) => workspace.id === workspaceId)?.folder}
          </p>
          <div className="local-columns">
            <div className="local-task-list">
              <div className="local-heading">
                <h3>Tarefas locais</h3>
                <button
                  className="button"
                  disabled={loading || busy}
                  onClick={() => openTask(null)}
                >
                  Nova tarefa
                </button>
              </div>
              {!loading && tasks.length === 0 && (
                <p>Nenhuma tarefa ainda. Registre o próximo passo do projeto.</p>
              )}
              {tasks.map((task) => (
                <button
                  key={task.id}
                  className={`local-task ${selected?.id === task.id ? "is-selected" : ""}`}
                  aria-pressed={selected?.id === task.id}
                  disabled={busy}
                  onClick={() => openTask(task)}
                >
                  <strong>{task.title}</strong>
                  <span>#{task.id} · Tarefa local</span>
                </button>
              ))}
            </div>
            {editing ? (
              <form
                className="local-form task-editor"
                onSubmit={async (event) => {
                  event.preventDefault();
                  if (workspaceId === null) return;
                  setBusy(true);
                  setError("");
                  setNotice("");
                  try {
                    const task = await saveTask({
                      ...form,
                      workspaceId,
                      id: selected?.id ?? null,
                      revision: selected?.revision ?? null,
                    });
                    setSelected(task);
                    setForm({
                      title: task.title,
                      description: task.description,
                      criteria: task.criteria,
                    });
                    setTasks((rows) => [task, ...rows.filter((row) => row.id !== task.id)]);
                    setNotice("Tarefa salva neste dispositivo.");
                  } catch (cause) {
                    setError(message(cause));
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                <h3>{selected ? "Editar tarefa" : "Nova tarefa"}</h3>
                <label>
                  Título
                  <input
                    id="task-title"
                    required
                    maxLength={300}
                    value={form.title}
                    disabled={busy}
                    onChange={(event) => setForm({ ...form, title: event.target.value })}
                  />
                </label>
                <label>
                  Descrição
                  <textarea
                    rows={6}
                    maxLength={100000}
                    value={form.description}
                    disabled={busy}
                    onChange={(event) => setForm({ ...form, description: event.target.value })}
                  />
                </label>
                <label>
                  Critérios de aceite
                  <textarea
                    rows={5}
                    maxLength={100000}
                    value={form.criteria}
                    disabled={busy}
                    onChange={(event) => setForm({ ...form, criteria: event.target.value })}
                  />
                </label>
                <div className="local-heading">
                  <span>
                    {dirty
                      ? "Alterações não salvas"
                      : selected
                        ? "Salva localmente"
                        : "Preencha os dados da tarefa"}
                  </span>
                  <button className="button primary" disabled={busy || !form.title.trim()}>
                    {busy ? "Salvando…" : "Salvar tarefa"}
                  </button>
                </div>
              </form>
            ) : (
              <div className="local-empty">
                <h3>Qual é o próximo passo?</h3>
                <p>
                  Abra uma tarefa para editar ou crie uma nova. As tarefas só são salvas ao clicar
                  em Salvar tarefa.
                </p>
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
}
