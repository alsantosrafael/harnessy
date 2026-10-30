# Criar retry

Crie `src/retry.js` exportando `retry(fn, attempts)`, que chama a função assíncrona `fn` até ela resolver.
- Se `fn` lançar erro, tenta de novo, até `attempts` chamadas no total.
- Se todas falharem, rejeita com o erro da última tentativa.
- `attempts` precisa ser inteiro maior ou igual a 1, senão lança `RangeError` de forma síncrona.
