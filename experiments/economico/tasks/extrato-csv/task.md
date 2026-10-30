# Exportação CSV quebra com vírgulas e aspas

O extrato exportado em CSV fica com colunas trocadas quando a descrição tem vírgula, aspas ou quebra de linha. Gere CSV conforme a RFC 4180.

## Critérios de aceitação
- Campo com vírgula, aspas ou quebra de linha vai entre aspas duplas; aspas internas são duplicadas.
- Campos sem esses caracteres saem sem aspas.
- Linhas separadas por `\r\n`; cabeçalho `data,descricao,valor` na primeira linha.
- Valores com 2 casas decimais e ponto como separador.
