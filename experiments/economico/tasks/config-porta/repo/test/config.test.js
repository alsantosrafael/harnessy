import { expect, test } from "bun:test";
import { loadConfig } from "../src/config.js";

test("padrões sem variáveis", () => {
  expect(loadConfig({})).toEqual({ host: "127.0.0.1", port: 3000 });
});
test("porta válida", () => {
  expect(loadConfig({ PORT: "8080" }).port).toBe(8080);
});
test("porta não numérica é recusada", () => {
  expect(() => loadConfig({ PORT: "abc" })).toThrow("PORT");
});
