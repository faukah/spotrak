import { formatChartValue } from "../stats/format";

export type ChartMetric = "count" | "duration";

export const CHART_PALETTE = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
  "var(--chart-6)",
  "var(--chart-7)",
  "var(--chart-8)",
];

export function chartColor(index: number): string {
  return CHART_PALETTE[index % CHART_PALETTE.length];
}

export function tickStep(length: number, maxTicks = 6): number {
  return Math.max(1, Math.ceil(length / maxTicks));
}

export function numericValue(value: unknown): number {
  if (typeof value === "number") return Number.isFinite(value) ? value : 0;
  if (typeof value === "string") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }
  return 0;
}

export function formatMetricValue(
  value: unknown,
  metric: ChartMetric,
): string {
  return formatChartValue(numericValue(value), metric);
}

export function formatCountValue(value: unknown): string {
  return formatMetricValue(value, "count");
}

export function formatDurationValue(value: unknown): string {
  return formatMetricValue(value, "duration");
}

export function formatPercentValue(value: unknown): string {
  const numeric = numericValue(value);
  return `${
    numeric.toLocaleString(undefined, {
      maximumFractionDigits: 1,
      minimumFractionDigits: numeric > 0 && numeric < 1 ? 1 : 0,
    })
  }%`;
}

export function formatShortDate(value: unknown): string {
  return formatDateLike(value, false);
}

export function formatLongDate(value: unknown): string {
  return formatDateLike(value, true);
}

function formatDateLike(value: unknown, long: boolean): string {
  if (value === null || value === undefined) return "";
  const raw = dateInput(value);
  if (raw === null) return "";
  const date = raw instanceof Date ? raw : new Date(raw);
  if (Number.isNaN(date.getTime())) return fallbackDateLabel(raw);
  return date.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: long ? "numeric" : undefined,
  });
}

function dateInput(value: unknown): string | number | Date | null {
  if (
    typeof value === "string" || typeof value === "number" ||
    value instanceof Date
  ) return value;
  return null;
}

function fallbackDateLabel(value: string | number | Date): string {
  if (typeof value === "string") return value;
  if (typeof value === "number") return value.toLocaleString();
  return value.toISOString();
}
