# Última página da listagem some

Usuários relatam que, quando a lista tem itens que não completam a última página, essa página não aparece e `pageCount` fica um a menos. Corrija.

## Critérios de aceitação
- `pageCount(total, size)` devolve o número de páginas arredondado para cima; lista vazia tem 0 páginas.
- `page(items, n, size)` devolve os itens da página `n` (começando em 1); página fora do intervalo devolve lista vazia.
- `size` menor que 1 lança `Error`.
