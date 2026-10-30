import { expect, test } from "bun:test";
import { page } from "../src/list.js";
import { pageCount } from "../src/paginate.js";

test("múltiplo exato não cria página vazia", () => {
  expect(pageCount(4, 2)).toBe(2);
  expect(page(["a", "b", "c", "d"], 3, 2)).toEqual([]);
});
test("lista vazia tem zero páginas", () => {
  expect(pageCount(0, 10)).toBe(0);
  expect(page([], 1, 10)).toEqual([]);
});
test("página zero ou negativa é vazia", () => {
  expect(page(["a"], 0, 1)).toEqual([]);
  expect(page(["a"], -1, 1)).toEqual([]);
});
test("tamanho inválido lança erro", () => {
  expect(() => pageCount(3, 0)).toThrow();
  expect(() => page(["a"], 1, 0)).toThrow();
});
