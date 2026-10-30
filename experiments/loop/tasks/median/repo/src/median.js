export function median(numbers) {
  if (numbers.length === 0) return null;
  const sorted = numbers.sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}
