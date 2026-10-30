/** Arredonda para centavos. */
export function round2(value) {
  return Math.round(value * 100) / 100;
}

/** Desconto em reais de um cupom sobre um subtotal. */
export function discountFor(subtotal, coupon) {
  if (!coupon) return 0;
  if (coupon.type === "fixed") return Math.min(coupon.value, subtotal);
  throw new Error(`cupom inválido: tipo ${coupon.type}`);
}
