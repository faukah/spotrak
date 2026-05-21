import { writable } from "svelte/store";
import type { StatsRangeKey } from "../api/types";

export interface StatsRangeSelection {
  range: StatsRangeKey;
  year?: number;
}

export const STATS_RANGE_STORAGE_KEY = "spotrak.statsRange";
export const STATS_RANGE_COOKIE_NAME = "spotrak_stats_range";

const STATS_RANGE_COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 365;

export const statsRangeOptions: { key: StatsRangeKey; label: string }[] = [
  { key: "today", label: "Today" },
  { key: "week", label: "This week" },
  { key: "month", label: "This month" },
  { key: "year", label: "This year" },
  { key: "all", label: "All time" },
];

const defaultSelection: StatsRangeSelection = { range: "all" };

export const selectedStatsRange = writable<StatsRangeSelection>(
  initialStatsRangeSelection(),
);

if (typeof window !== "undefined") {
  selectedStatsRange.subscribe((selection) => {
    const normalized = normalizeStatsRangeSelection(selection);
    const serialized = JSON.stringify(normalized);
    window.localStorage.setItem(STATS_RANGE_STORAGE_KEY, serialized);
    document.cookie = [
      `${STATS_RANGE_COOKIE_NAME}=${encodeURIComponent(serialized)}`,
      "Path=/",
      `Max-Age=${STATS_RANGE_COOKIE_MAX_AGE_SECONDS}`,
      "SameSite=Lax",
    ].join("; ");
  });

  window.addEventListener("storage", (event) => {
    if (event.key !== STATS_RANGE_STORAGE_KEY) return;
    selectedStatsRange.set(parseStatsRangeSelection(event.newValue));
  });
}

export function normalizeStatsRangeSelection(
  selection: StatsRangeSelection,
): StatsRangeSelection {
  if (selection.range === "selected-year") {
    return {
      range: "selected-year",
      year: validYear(selection.year)
        ? selection.year
        : new Date().getFullYear(),
    };
  }
  return { range: selection.range };
}

export function statsRangeLabel(selection: StatsRangeSelection): string {
  if (selection.range === "selected-year") {
    return String(selection.year ?? new Date().getFullYear());
  }
  return statsRangeOptions.find((option) => option.key === selection.range)
    ?.label ??
    "All time";
}

export function statsRangeQuery(selection: StatsRangeSelection): string {
  const normalized = normalizeStatsRangeSelection(selection);
  const params = new URLSearchParams({ range: normalized.range });
  if (normalized.range === "selected-year" && normalized.year) {
    params.set("year", String(normalized.year));
  }
  return params.toString();
}

export function statsRangeSelectionKey(selection: StatsRangeSelection): string {
  const normalized = normalizeStatsRangeSelection(selection);
  return `${normalized.range}:${normalized.year ?? ""}`;
}

export function isStatsRangeKey(value: unknown): value is StatsRangeKey {
  return value === "today" || value === "week" || value === "month" ||
    value === "year" || value === "selected-year" || value === "all";
}

function initialStatsRangeSelection(): StatsRangeSelection {
  if (typeof window === "undefined") return defaultSelection;
  return parseStatsRangeSelection(
    window.localStorage.getItem(STATS_RANGE_STORAGE_KEY),
  );
}

export function parseStatsRangeSelection(
  value: string | null,
): StatsRangeSelection {
  if (!value) return defaultSelection;
  try {
    const parsed: unknown = JSON.parse(value);
    if (!isSelectionRecord(parsed) || !isStatsRangeKey(parsed.range)) {
      return defaultSelection;
    }
    return normalizeStatsRangeSelection({
      range: parsed.range,
      year: validYear(parsed.year) ? parsed.year : undefined,
    });
  } catch {
    return defaultSelection;
  }
}

function isSelectionRecord(
  value: unknown,
): value is { range?: unknown; year?: unknown } {
  return typeof value === "object" && value !== null;
}

function validYear(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) &&
    value >= 1900 && value <= 3000;
}
