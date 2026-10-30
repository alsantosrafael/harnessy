import { expect, test } from "bun:test";
import { loadConfig } from "../src/config.js";

test("limites da faixa", () => {
  expect(loadConfig({ PORT: "1" }).port).toBe(1);
  expect(loadConfig({ PORT: "65535" }).port).toBe(65535);
  expect(() => loadConfig({ PORT: "0" })).toThrow("PORT");
  expect(() => loadConfig({ PORT: "65536" })).toThrow("PORT");
});
test("espaços nas pontas são ignorados", () => {
  expect(loadConfig({ PORT: " 9000 " }).port).toBe(9000);
});
test("lixo depois do número é recusado", () => {
  expect(() => loadConfig({ PORT: "80abc" })).toThrow("80abc");
  expect(() => loadConfig({ PORT: "8.5" })).toThrow("PORT");
  expect(() => loadConfig({ PORT: "" })).toThrow("PORT");
});
test("host continua independente", () => {
  expect(loadConfig({ HOST: "0.0.0.0", PORT: "443" })).toEqual({ host: "0.0.0.0", port: 443 });
});
