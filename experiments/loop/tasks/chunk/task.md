# Criar chunk

Crie `src/chunk.js` exportando `chunk(items, size)`, que divide um array em pedaços de tamanho `size`.
- O último pedaço pode ser menor.
- `size` precisa ser inteiro positivo, senão lança `RangeError`.
- Array vazio devolve `[]`. A entrada não pode ser alterada.
