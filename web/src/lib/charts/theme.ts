import { formatChartValue } from "../stats/format";

export type ChartMetric = "count" | "duration";

export interface ChartColors {
  text: string;
  muted: string;
  border: string;
  primary: string;
  accent: string;
  panel: string;
}

export const CHART_PALETTE = [
  "#9eb98e",
  "#d1a75f",
  "#7fa0b8",
  "#bd8b7b",
  "#a68cc2",
  "#7eb59f",
  "#c7c176",
  "#8490a8",
];

export function chartColors(): ChartColors {
  const styles = getComputedStyle(document.documentElement);
  return {
    text: cssVar(styles, "--color-text"),
    muted: cssVar(styles, "--color-muted"),
    border: cssVar(styles, "--color-border"),
    primary: cssVar(styles, "--color-primary"),
    accent: cssVar(styles, "--color-accent"),
    panel: cssVar(styles, "--color-panel"),
  };
}

export function chartTooltip(
  colors: ChartColors,
  valueFormatter?: (value: number) => string,
) {
  return {
    confine: true,
    backgroundColor: colors.panel,
    borderColor: colors.border,
    textStyle: { color: colors.text, fontFamily: "var(--font-sans)" },
    ...(valueFormatter ? { valueFormatter } : {}),
  };
}

export function metricTooltip(colors: ChartColors, metric: ChartMetric) {
  return chartTooltip(
    colors,
    (value: number) => formatChartValue(value, metric),
  );
}

function cssVar(styles: CSSStyleDeclaration, name: string): string {
  return styles.getPropertyValue(name).trim();
}
