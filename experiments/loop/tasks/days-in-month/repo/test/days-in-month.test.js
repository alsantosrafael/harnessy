import { expect, test } from "bun:test";
import { daysInMonth } from "../src/days-in-month.js";

test("fevereiro de ano bissexto comum", () => {
  expect(daysInMonth(2024, 2)).toBe(29);
});
test("fevereiro de século que não é bissexto", () => {
  expect(daysInMonth(1900, 2)).toBe(28);
});
test("fevereiro de século bissexto", () => {
  expect(daysInMonth(2000, 2)).toBe(29);
});
test("meses de 30 e 31 dias", () => {
  expect(daysInMonth(2023, 4)).toBe(30);
  expect(daysInMonth(2023, 12)).toBe(31);
});
test("mês inválido", () => {
  expect(() => daysInMonth(2023, 13)).toThrow(RangeError);
});
