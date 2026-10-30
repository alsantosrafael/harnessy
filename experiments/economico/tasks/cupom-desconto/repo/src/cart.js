import { discountFor, round2 } from "./pricing.js";

/** Soma `price * qty` dos itens e aplica o cupom, se houver. */
export function cartTotal(items, coupon) {
  const subtotal = items.reduce((sum, item) => sum + item.price * item.qty, 0);
  return round2(subtotal - discountFor(subtotal, coupon));
}
