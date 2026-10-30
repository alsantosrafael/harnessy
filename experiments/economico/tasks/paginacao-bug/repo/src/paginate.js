/** Número de páginas para `total` itens com `size` por página. */
export function pageCount(total, size) {
  return Math.floor(total / size);
}
