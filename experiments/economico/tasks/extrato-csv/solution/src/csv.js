/** Campo CSV conforme RFC 4180: aspas só quando necessário, aspas internas duplicadas. */
function field(value) {
  const text = String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

/** Junta campos de uma linha CSV. */
export function csvLine(fields) {
  return fields.map(field).join(",");
}
