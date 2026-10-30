import { expect, test } from "bun:test";
import { page } from "../src/list.js";
import { pageCount } from "../src/paginate.js";

const letters = ["a", "b", "c", "d", "e"];

test("primeira página", () => {
  expect(page(letters, 1, 2)).toEqual(["a", "b"]);
});
test("última página incompleta aparece", () => {
  expect(pageCount(5, 2)).toBe(3);
  expect(page(letters, 3, 2)).toEqual(["e"]);
});
