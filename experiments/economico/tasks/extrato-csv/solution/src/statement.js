import { csvLine } from "./csv.js";

/** Extrato em CSV: cabeçalho e uma linha por lançamento, separados por CRLF. */
export function statementCsv(entries) {
  const lines = [csvLine(["data", "descricao", "valor"])];
  for (const entry of entries) {
    lines.push(csvLine([entry.date, entry.description, entry.amount.toFixed(2)]));
  }
  return lines.join("\r\n");
}
