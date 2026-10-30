# Cupom de desconto percentual com teto

O carrinho já soma itens e aplica cupom de valor fixo. Adicione cupom percentual.

Um cupom percentual tem a forma `{ type: "percent", value, cap }`: `value` é a porcentagem (0 a 100) e `cap` é o desconto máximo em reais (opcional). O desconto nunca deixa o total negativo. Totais são arredondados para 2 casas.

## Critérios de aceitação
- `cartTotal(items, { type: "percent", value: 10 })` aplica 10% sobre a soma dos itens.
- Com `cap`, o desconto percentual não passa do teto.
- Cupom percentual com `value` fora de 0–100 lança `Error` com a mensagem contendo "cupom inválido".
- O cupom fixo continua funcionando como antes.
