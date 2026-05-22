import { formatChartValue } from "../stats/format";

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

export function numericValue(value: unknown): number {
  if (typeof value === "number") return Number.isFinite(value) ? value : 0;
  if (typeof value === "string") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }
  return 0;
}

export function formatCountValue(value: unknown): string {
  return formatChartValue(numericValue(value), "count");
}

export function formatDurationValue(value: unknown): string {
  return formatChartValue(numericValue(value), "duration");
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
