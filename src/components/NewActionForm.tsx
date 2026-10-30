import { useState } from "react";
import { suggestId, TASK_VARIABLES } from "../lib/actions";
import { agentOf } from "../lib/agents";
import { type ActionInfo, createAction, type NewAction } from "../lib/desktop";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** Modelos aceitos pela Ação: os do Claude Code, único runtime com adaptador. */
const MODELS = agentOf("claude")?.models ?? [];

const EMPTY: NewAction = {
  id: "",
  nome: "",
  descricao: "",
  acesso: "leitura",
  modelo: null,
  pedido: "",
};

/** Formulário de Ação do usuário: o resultado é um arquivo TOML; ninguém precisa escrevê-lo. */
export function NewActionForm({ onCreated }: { onCreated: (action: ActionInfo) => void }) {
  const [form, setForm] = useState(EMPTY);
  const [idTouched, setIdTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const id = idTouched ? form.id : suggestId(form.nome);
  const ready = id !== "" && form.nome.trim() !== "" && form.pedido.trim() !== "";

  return (
    <form
      className="local-form new-action"
      onSubmit={(event) => {
        event.preventDefault();
        setBusy(true);
        setError("");
        createAction({ ...form, id })
          .then((created) => {
            setForm(EMPTY);
            setIdTouched(false);
            onCreated(created);
          })
          .catch((cause) => setError(message(cause)))
          .finally(() => setBusy(false));
      }}
    >
      <label>
        Nome
        <input
          value={form.nome}
          maxLength={80}
          disabled={busy}
          onChange={(event) => setForm({ ...form, nome: event.target.value })}
        />
      </label>
      <label>
        Id (nome do arquivo)
        <input
          value={id}
          maxLength={40}
          disabled={busy}
          onChange={(event) => {
            setIdTouched(true);
            setForm({ ...form, id: event.target.value });
          }}
        />
      </label>
      <label>
        Descrição
        <input
          value={form.descricao}
          maxLength={200}
          disabled={busy}
          onChange={(event) => setForm({ ...form, descricao: event.target.value })}
        />
      </label>
      <label>
        Acesso
        <select
          value={form.acesso}
          disabled={busy}
          onChange={(event) =>
            setForm({ ...form, acesso: event.target.value as NewAction["acesso"] })
          }
        >
          <option value="leitura">Só leitura (responde, não altera)</option>
          <option value="escrita">Altera o repositório (com verificações)</option>
        </select>
      </label>
      <label>
        Modelo
        <select
          value={form.modelo ?? ""}
          disabled={busy}
          onChange={(event) => setForm({ ...form, modelo: event.target.value || null })}
        >
          <option value="">O escolhido na tarefa</option>
          {MODELS.map((model) => (
            <option key={model.id} value={model.id}>
              {model.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        Pedido ao agente
        <textarea
          rows={5}
          maxLength={8000}
          value={form.pedido}
          disabled={busy}
          onChange={(event) => setForm({ ...form, pedido: event.target.value })}
        />
      </label>
      <p className="more-actions-note">
        Variáveis:{" "}
        {TASK_VARIABLES.map((name) => (
          <button
            key={name}
            type="button"
            className="button"
            disabled={busy}
            onClick={() => setForm({ ...form, pedido: `${form.pedido}{{${name}}}` })}
          >
            {`{{${name}}}`}
          </button>
        ))}
      </p>
      {error && (
        <p className="local-error" role="alert">
          {error}
        </p>
      )}
      <button className="button primary" disabled={busy || !ready}>
        {busy ? "Criando…" : "Criar Ação"}
      </button>
    </form>
  );
}
