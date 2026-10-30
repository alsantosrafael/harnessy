const DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

export function daysInMonth(year, month) {
  if (month < 1 || month > 12) throw new RangeError("mês inválido");
  if (month === 2 && year % 4 === 0) return 29;
  return DAYS[month - 1];
}
