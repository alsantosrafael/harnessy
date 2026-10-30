import { expect, test } from "bun:test";
import { formatBytes } from "../src/format-bytes.js";

test("bytes abaixo de 1 KB", () => {
  expect(formatBytes(1023)).toBe("1023 B");
});
test("kilobytes com uma casa decimal", () => {
  expect(formatBytes(1536)).toBe("1.5 KB");
});
test("sem casa decimal quando inteiro", () => {
  expect(formatBytes(10240)).toBe("10 KB");
  expect(formatBytes(1048576)).toBe("1 MB");
});
test("gigabytes", () => {
  expect(formatBytes(3221225472)).toBe("3 GB");
});
test("valores inválidos", () => {
  expect(() => formatBytes(-1)).toThrow(RangeError);
  expect(() => formatBytes(Number.NaN)).toThrow(RangeError);
});
