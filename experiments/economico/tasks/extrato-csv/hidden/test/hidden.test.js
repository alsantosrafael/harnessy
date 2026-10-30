import { expect, test } from "bun:test";
import { statementCsv } from "../src/statement.js";

const line = (description) =>
  statementCsv([{ date: "2026-02-01", description, amount: -3 }]).split("\r\n").slice(1).join("\r\n");

test("aspas internas são duplicadas", () => {
  expect(line('Livro "Rust"')).toBe('2026-02-01,"Livro ""Rust""",-3.00');
});
test("quebra de linha fica dentro das aspas", () => {
  expect(line("linha 1\nlinha 2")).toBe('2026-02-01,"linha 1\nlinha 2",-3.00');
});
test("campo comum continua sem aspas", () => {
  expect(line("Mercado")).toBe("2026-02-01,Mercado,-3.00");
});
test("vários lançamentos, separador CRLF", () => {
  const csv = statementCsv([
    { date: "d1", description: "a", amount: 1 },
    { date: "d2", description: "b,c", amount: 2.345 },
  ]);
  expect(csv).toBe('data,descricao,valor\r\nd1,a,1.00\r\nd2,"b,c",2.35');
});
