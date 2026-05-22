import { formatDuration } from "../date/format";

export function formatPercent(value: number | null | undefined, digits = 0): string {
  const safeValue = typeof value === "number" && Number.isFinite(value) ? value : 0;
  return `${safeValue.toFixed(digits)}%`;
}

export function formatNumber(value: number | null | undefined, digits = 0): string {
  const safeValue = typeof value === "number" && Number.isFinite(value) ? value : 0;
  return safeValue.toLocaleString(undefined, {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits,
  });
}

export function formatGap(ms: number | null | undefined): string {
  const safeMs = typeof ms === "number" && Number.isFinite(ms) ? ms : 0;
  const days = Math.floor(safeMs / 86_400_000);
  if (days >= 365) {
    const years = days / 365;
    return `${years.toFixed(years >= 10 ? 0 : 1)}y`;
  }
  if (days >= 30) {
    const months = days / 30;
    return `${months.toFixed(months >= 10 ? 0 : 1)}mo`;
  }
  if (days >= 1) return `${days}d`;
  return formatDuration(safeMs);
}

export function formatDate(value: string, timeZone?: string | null): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeZone: timeZone ?? undefined,
  }).format(new Date(value));
}
