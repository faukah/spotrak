export function formatChartValue(
  value: number,
  metric: "count" | "duration",
): string {
  if (metric === "count") return Intl.NumberFormat().format(Math.round(value));
  const minutes = value / 60_000;
  if (minutes < 60) return `${Math.round(minutes)}m`;
  const hours = minutes / 60;
  if (hours < 24) return `${hours.toFixed(hours < 10 ? 1 : 0)}h`;
  const days = hours / 24;
  return `${days.toFixed(days < 10 ? 1 : 0)}d`;
}
