import assert from "node:assert/strict";
import { test } from "vitest";
import type { ActionInfo } from "../src/lib/desktop.ts";
import {
  accessLabel,
  extraActions,
  originLabel,
  suggestId,
  unavailableReason,
} from "../src/lib/actions.ts";

const action = (id: string, extra: Partial<ActionInfo> = {}): ActionInfo => ({
  id,
  nome: id,
  descricao: `Faz ${id}.`,
  acesso: "leitura",
  alvo: "tarefa",
  saida: "texto",
  origem: "embutida",
  pendente: false,
  ...extra,
});

test("Mais ações lista tudo menos a principal, na ordem recebida", () => {
  const list = [action("implementar"), action("explicar"), action("minha")];
  assert.deepEqual(
    extraActions(list).map((item) => item.id),
    ["explicar", "minha"],
  );
  assert.deepEqual(extraActions([action("implementar")]), []);
});

test("Ações sobre run (revisor) não viram botão da tarefa", () => {
  const list = [action("revisar", { alvo: "run", saida: "revisao" }), action("dividir-em-passos")];
  assert.deepEqual(
    extraActions(list).map((item) => item.id),
    ["dividir-em-passos"],
  );
});

test("Ação com erro traz o motivo; sem erro, nada a mostrar", () => {
  assert.equal(unavailableReason(action("explicar")), null);
  const broken = action("quebrada", { erro: 'Ação inválida: linha 3: campo desconhecido "alvo"' });
  assert.equal(unavailableReason(broken), 'Ação inválida: linha 3: campo desconhecido "alvo"');
});

test("rótulo de acesso descreve leitura, escrita e Ação inválida", () => {
  assert.equal(accessLabel("leitura"), "Só leitura");
  assert.equal(accessLabel("escrita"), "Altera o repositório");
  assert.equal(accessLabel(null), "");
});

test("id sugerido vem do nome, sem acento e seguro para arquivo", () => {
  assert.equal(suggestId("Adicionar Testes já!"), "adicionar-testes-ja");
  assert.equal(suggestId("  ---  "), "");
  assert.ok(suggestId("a".repeat(80)).length <= 40);
});

test("origem tem rótulo curto", () => {
  assert.equal(originLabel("repositorio"), "Do repositório");
  assert.equal(originLabel("usuario"), "Sua");
  assert.equal(originLabel("embutida"), "Embutida");
});
