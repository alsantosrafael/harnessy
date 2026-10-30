/** Arredonda para centavos. */
export function round2(value) {
  return Math.round(value * 100) / 100;
}

/** Desconto em reais de um cupom sobre um subtotal. */
export function discountFor(subtotal, coupon) {
  if (!coupon) return 0;
  if (coupon.type === "fixed") return Math.min(coupon.value, subtotal);
  if (coupon.type === "percent") {
    if (!(coupon.value >= 0 && coupon.value <= 100)) {
      throw new Error(`cupom inválido: porcentagem ${coupon.value}`);
    }
    const raw = (subtotal * coupon.value) / 100;
    const capped = coupon.cap === undefined ? raw : Math.min(raw, coupon.cap);
    return Math.min(capped, subtotal);
  }
  throw new Error(`cupom inválido: tipo ${coupon.type}`);
}
