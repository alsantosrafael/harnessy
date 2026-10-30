# Corrigir daysInMonth

`src/days-in-month.js` exporta `daysInMonth(year, month)`, mas trata todo ano divisível por 4 como bissexto. Corrija a regra do ano bissexto.
- Regra: divisível por 4, exceto séculos, que só são bissextos se divisíveis por 400.
- `month` de 1 a 12, senão lança `RangeError`.
