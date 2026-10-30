import { expect, test } from "bun:test";
import { parseDuration } from "../src/parse-duration.js";

test("horas e minutos", () => {
  expect(parseDuration("1h30m")).toBe(5400);
});
test("dias", () => {
  expect(parseDuration("2d")).toBe(172800);
});
test("segundos", () => {
  expect(parseDuration("45s")).toBe(45);
  expect(parseDuration("0s")).toBe(0);
});
test("ordem livre", () => {
  expect(parseDuration("30m1h")).toBe(5400);
});
test("entrada inválida", () => {
  expect(() => parseDuration("")).toThrow();
  expect(() => parseDuration("10x")).toThrow();
  expect(() => parseDuration("10")).toThrow();
  expect(() => parseDuration("1h1h")).toThrow();
});
