import { expect, test } from "bun:test";
import { median } from "../src/median.js";

test("ímpar", () => {
  expect(median([3, 1, 2])).toBe(2);
});
test("par faz a média dos dois centrais", () => {
  expect(median([4, 1, 2, 3])).toBe(2.5);
});
test("não altera a entrada", () => {
  const numbers = [5, 1, 3];
  median(numbers);
  expect(numbers).toEqual([5, 1, 3]);
});
test("vazio", () => {
  expect(median([])).toBeNull();
});
