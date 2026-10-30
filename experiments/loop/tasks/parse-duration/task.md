# Criar parseDuration

Crie `src/parse-duration.js` exportando `parseDuration(text)`, que converte uma duração como `1h30m` em segundos.
- Unidades aceitas: `d` (dia), `h`, `m` (minuto), `s`. Podem aparecer em qualquer ordem, e cada uma uma vez no máximo.
- `0s` vale 0.
- Qualquer outro texto (vazio, unidade desconhecida, número sem unidade) lança `Error`.
