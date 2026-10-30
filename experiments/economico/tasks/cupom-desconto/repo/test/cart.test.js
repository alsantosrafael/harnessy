import { expect, test } from "bun:test";
import { cartTotal } from "../src/cart.js";

const items = [
  { price: 50, qty: 2 },
  { price: 25, qty: 1 },
];

test("sem cupom soma os itens", () => {
  expect(cartTotal(items)).toBe(125);
});
test("cupom fixo desconta o valor", () => {
  expect(cartTotal(items, { type: "fixed", value: 20 })).toBe(105);
});
test("cupom percentual desconta a porcentagem", () => {
  expect(cartTotal(items, { type: "percent", value: 10 })).toBe(112.5);
});
