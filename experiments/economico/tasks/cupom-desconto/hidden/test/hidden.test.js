import { expect, test } from "bun:test";
import { cartTotal } from "../src/cart.js";

const items = [
  { price: 50, qty: 2 },
  { price: 25, qty: 1 },
];

test("teto limita o desconto percentual", () => {
  expect(cartTotal(items, { type: "percent", value: 50, cap: 30 })).toBe(95);
});
test("teto maior que o desconto não muda nada", () => {
  expect(cartTotal(items, { type: "percent", value: 10, cap: 100 })).toBe(112.5);
});
test("arredonda para centavos", () => {
  expect(cartTotal([{ price: 9.99, qty: 3 }], { type: "percent", value: 15 })).toBe(25.47);
});
test("100% zera sem ficar negativo", () => {
  expect(cartTotal(items, { type: "percent", value: 100 })).toBe(0);
});
test("porcentagem fora de 0–100 é inválida", () => {
  expect(() => cartTotal(items, { type: "percent", value: 120 })).toThrow("cupom inválido");
  expect(() => cartTotal(items, { type: "percent", value: -5 })).toThrow("cupom inválido");
});
test("cupom fixo maior que o subtotal zera", () => {
  expect(cartTotal(items, { type: "fixed", value: 500 })).toBe(0);
});
