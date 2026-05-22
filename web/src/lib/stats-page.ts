import type { StatsBucketAxis, StatsRangeKey } from "./api/types";
import {
  parseStatsRangeSelection,
  statsRangeQuery,
  type StatsRangeSelection,
} from "./stores/stats-range";

export function statsRangeFromCookie(
  value: string | null,
): StatsRangeSelection {
  return parseStatsRangeSelection(decodeCookieValue(value));
}

export function overviewQuery(selection: StatsRangeSelection): string {
  return statsRangeQuery(selection);
}

export function dashboardQuery(selection: StatsRangeSelection): string {
  const params = new URLSearchParams(statsRangeQuery(selection));
  params.set("split", splitForStatsRange(selection.range));
  return params.toString();
}

export function splitForStatsRange(
  range: StatsRangeKey,
): StatsBucketAxis["split"] {
  if (range === "today") return "hour";
  if (range === "week" || range === "month") return "day";
  if (range === "year" || range === "selected-year") return "week";
  return "month";
}

function decodeCookieValue(value: string | null): string | null {
  if (!value) return null;
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}
