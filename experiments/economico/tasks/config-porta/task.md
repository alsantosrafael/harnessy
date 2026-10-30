# Validar a porta na configuração

`loadConfig(env)` lê a configuração das variáveis de ambiente. Hoje um `PORT` inválido vira `NaN` e o servidor sobe quebrado. Valide a porta.

## Critérios de aceitação
- Sem `PORT`, a porta é 3000.
- `PORT` precisa ser um inteiro entre 1 e 65535; espaços nas pontas são ignorados.
- `PORT` inválido lança `Error` cuja mensagem contém "PORT" e o valor recebido.
- `HOST` continua com padrão `127.0.0.1`.
