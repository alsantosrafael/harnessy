import { pageCount } from "./paginate.js";

/** Itens da página `n` (1-based). */
export function page(items, n, size) {
  const pages = pageCount(items.length, size);
  if (n < 1 || n > pages) return [];
  return items.slice((n - 1) * size, n * size);
}
