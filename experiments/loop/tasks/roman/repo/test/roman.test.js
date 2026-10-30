import { expect, test } from "bun:test";
import { toRoman } from "../src/roman.js";

test("subtrativos", () => {
  expect(toRoman(4)).toBe("IV");
  expect(toRoman(1994)).toBe("MCMXCIV");
});
test("limite superior", () => {
  expect(toRoman(3999)).toBe("MMMCMXCIX");
});
test("limite inferior", () => {
  expect(toRoman(1)).toBe("I");
});
test("fora do intervalo", () => {
  expect(() => toRoman(0)).toThrow(RangeError);
  expect(() => toRoman(4000)).toThrow(RangeError);
  expect(() => toRoman(2.5)).toThrow(RangeError);
});
