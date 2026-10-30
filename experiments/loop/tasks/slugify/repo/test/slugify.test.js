import { expect, test } from "bun:test";
import { slugify } from "../src/slugify.js";

test("converte frase com pontuação", () => {
  expect(slugify("Olá, Mundo!")).toBe("ola-mundo");
});
test("colapsa espaços e símbolos repetidos", () => {
  expect(slugify("  Multiple   spaces__here ")).toBe("multiple-spaces-here");
});
test("remove acentos", () => {
  expect(slugify("Ação & Ótimo")).toBe("acao-otimo");
});
test("texto vazio", () => {
  expect(slugify("")).toBe("");
});
