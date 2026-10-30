import { expect, test } from "bun:test";
import { chunk } from "../src/chunk.js";

test("divide com último pedaço menor", () => {
  expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
});
test("array vazio", () => {
  expect(chunk([], 3)).toEqual([]);
});
test("não altera a entrada", () => {
  const items = [1, 2, 3];
  chunk(items, 2);
  expect(items).toEqual([1, 2, 3]);
});
test("tamanho inválido", () => {
  expect(() => chunk([1], 0)).toThrow(RangeError);
  expect(() => chunk([1], -1)).toThrow(RangeError);
  expect(() => chunk([1], 1.5)).toThrow(RangeError);
});
