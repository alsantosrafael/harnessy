# Criar slugify

Crie `src/slugify.js` exportando `slugify(text)`: converte o texto em slug para URL.
- Minúsculas, sem acentos (use normalização NFD e remova as marcas diacríticas).
- Qualquer sequência de caracteres que não seja letra ou dígito vira um único hífen.
- Sem hífens no início ou no fim. Texto vazio devolve string vazia.
