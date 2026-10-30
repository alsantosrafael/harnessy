# Criar formatBytes

Crie `src/format-bytes.js` exportando `formatBytes(n)`, que formata um tamanho em bytes para leitura humana.
- Abaixo de 1024: inteiro seguido de ` B`, por exemplo `1023 B`.
- Acima disso: base 1024, unidades `KB`, `MB`, `GB`, `TB`, com uma casa decimal, sem `.0` final.
- Negativo, `NaN` ou infinito lançam `RangeError`.
