import { expect, test } from "bun:test";
import { retry } from "../src/retry.js";

test("devolve o valor da primeira tentativa bem-sucedida", async () => {
  expect(await retry(async () => 42, 3)).toBe(42);
});
test("tenta de novo até ter sucesso", async () => {
  let calls = 0;
  const value = await retry(async () => {
    calls += 1;
    if (calls < 3) throw new Error("falhou");
    return "ok";
  }, 3);
  expect(value).toBe("ok");
  expect(calls).toBe(3);
});
test("rejeita com o erro da última tentativa", async () => {
  let calls = 0;
  const error = await retry(async () => {
    calls += 1;
    throw new Error(`tentativa ${calls}`);
  }, 2).catch((reason) => reason);
  expect(error.message).toBe("tentativa 2");
  expect(calls).toBe(2);
});
test("attempts inválido", () => {
  expect(() => retry(async () => 1, 0)).toThrow(RangeError);
});
