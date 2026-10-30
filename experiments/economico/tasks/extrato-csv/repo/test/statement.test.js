import { expect, test } from "bun:test";
import { statementCsv } from "../src/statement.js";

test("extrato simples", () => {
  const csv = statementCsv([{ date: "2026-01-02", description: "Café", amount: 7.5 }]);
  expect(csv).toBe("data,descricao,valor\r\n2026-01-02,Café,7.50");
});
test("vírgula na descrição fica entre aspas", () => {
  const csv = statementCsv([{ date: "2026-01-03", description: "Pão, leite", amount: 12 }]);
  expect(csv.split("\r\n")[1]).toBe('2026-01-03,"Pão, leite",12.00');
});
