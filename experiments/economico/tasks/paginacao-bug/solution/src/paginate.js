/** Número de páginas para `total` itens com `size` por página. */
export function pageCount(total, size) {
  if (!(size >= 1)) throw new Error(`tamanho de página inválido: ${size}`);
  return Math.ceil(total / size);
}
