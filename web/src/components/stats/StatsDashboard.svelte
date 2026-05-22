<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { apiFetch } from "../../lib/api/client";
  import type {
    AlbumReleaseYearsStats,
    BucketedTopArtist,
    ComebackArtist,
    DiscoveryStats,
    DiversityTimelinePoint,
    FeatureAverageStats,
    FeatureTimelinePoint,
    HourRepartitionPoint,
    HourlyTopArtist,
    ListeningConcentrationStats,
    ListeningSessionStats,
    RepeatLoopStats,
    StatsBucketAxis,
    StatsDashboardBootstrapResponse,
    StatsDashboardResponse,
    SummaryStats,
    TimelinePoint,
  } from "../../lib/api/types";
  import {
    chartColor,
    formatCountValue,
    formatDurationValue,
    numericValue,
  } from "../../lib/charts/theme";
  import { formatDuration } from "../../lib/date/format";
  import { splitForStatsRange } from "../../lib/stats-page";
  import {
    normalizeStatsRangeSelection,
    selectedStatsRange,
    statsRangeLabel,
    statsRangeQuery,
    statsRangeSelectionKey,
    type StatsRangeSelection,
  } from "../../lib/stores/stats-range";
  import * as Card from "../ui/card";
  import LazyStatsPanel from "./LazyStatsPanel.svelte";
  import StatsRangePicker from "./StatsRangePicker.svelte";
  import SummaryRail from "./dashboard/SummaryRail.svelte";

  type TimeSplit = StatsBucketAxis["split"];
  type SummaryItem = {
    label: string;
    value: string;
    detail: string;
  };
  type StatsMetricPoint = {
    label: string;
    value: number;
    rawLabel?: string;
  };
  export let apiPrefix = "";
  export let pagePrefix = "";
  export let initialHourFormat: "12" | "24" = "24";
  export let initialBootstrap: StatsDashboardBootstrapResponse | null = null;
  export let initialRange: StatsRangeSelection | null = null;

  const currentYear = new Date().getFullYear();
  const defaultRange = normalizeStatsRangeSelection(
    initialRange ?? { range: initialBootstrap?.range?.range ?? "all" },
  );
  const hasBootstrap = initialBootstrap !== null;
  const loadArtistDistributionChart = () => import("./StatsArtistDistributionChart.svelte");
  const loadBucketMetricsChart = () => import("./StatsBucketMetricsChart.svelte");
  const loadDayDistributionChart = () => import("./StatsDayDistributionChart.svelte");
  const loadHourArtistHeatmap = () => import("./StatsHourArtistHeatmap.svelte");
  const loadMetricChart = () => import("./StatsMetricChart.svelte");
  const loadComebackArtistsPanel = () => import("./dashboard/ComebackArtistsPanel.svelte");
  const loadConcentrationPanel = () => import("./dashboard/ConcentrationPanel.svelte");
  const loadDiscoveryComfortCard = () => import("./dashboard/DiscoveryComfortCard.svelte");
  const loadListeningSessionsPanel = () => import("./dashboard/ListeningSessionsPanel.svelte");
  const loadRepeatLoopsPanel = () => import("./dashboard/RepeatLoopsPanel.svelte");

  let activeRange: StatsRangeSelection = defaultRange;
  let availableYears: number[] = initialBootstrap?.available_years.length
    ? initialBootstrap.available_years
    : [currentYear];
  let summary: SummaryStats | null = initialBootstrap?.summary ?? null;
  let discovery: DiscoveryStats | null = null;
  let artistDistributionRows: BucketedTopArtist[] = [];
  let hours: HourRepartitionPoint[] = [];
  let hourlyArtists: HourlyTopArtist[] = [];
  let timeline: TimelinePoint[] = [];
  let diversity: DiversityTimelinePoint[] = [];
  let releaseYears: AlbumReleaseYearsStats | null = initialBootstrap?.release_years ?? null;
  let featureAverage: FeatureAverageStats | null = initialBootstrap?.feature_average ?? null;
  let featureTimeline: FeatureTimelinePoint[] = [];
  let sessions: ListeningSessionStats | null = null;
  let concentration: ListeningConcentrationStats | null = null;
  let comebackArtists: ComebackArtist[] = [];
  let repeatLoops: RepeatLoopStats | null = null;
  let bucketAxis: string[] = initialBootstrap?.bucket_axis.buckets ?? [];
  let bucketAxisSplit: TimeSplit = initialBootstrap?.bucket_axis.split ??
    splitForStatsRange(activeRange.range);
  let hourFormat: "12" | "24" = initialBootstrap?.hour_format ?? initialHourFormat;
  let timezone: string | null = initialBootstrap?.timezone ?? null;
  let loading = initialBootstrap === null;
  let refreshing = false;
  let error: string | null = null;
  let requestId = 0;
  let unsubscribeRange: (() => void) | undefined;
  let stopReducedMotionWatch: (() => void) | undefined;
  let prefersReducedMotion = false;
  let fullDashboardLoaded = false;
  let lastRangeKey = statsRangeSelectionKey(activeRange);

  $: timelineSplit = bucketAxisSplit === "all" ? splitForStatsRange(activeRange.range) : bucketAxisSplit;
  $: timelineDescription = `${splitLabel(timelineSplit)} buckets`;
  $: activeRangeLabel = statsRangeLabel(activeRange);
  $: bucketKeys = bucketAxis;
  $: timelineByBucket = new Map(timeline.map((point) => [point.bucket, point]));
  $: diversityByBucket = new Map(
    diversity.map((point) => [point.bucket, point]),
  );
  $: listensPoints = bucketKeys.map((bucket) =>
    metricPoint(bucket, timelineByBucket.get(bucket)?.count ?? 0),
  );
  $: timePoints = bucketKeys.map((bucket) =>
    metricPoint(bucket, timelineByBucket.get(bucket)?.duration_ms ?? 0),
  );
  $: uniqueArtistPoints = bucketKeys.map((bucket) =>
    metricPoint(bucket, diversityByBucket.get(bucket)?.unique_artists ?? 0),
  );
  $: releaseYearPoints = diversity
    .filter(
      (point) =>
        point.average_release_year !== null &&
        point.average_release_year !== undefined,
    )
    .map((point) => metricPoint(point.bucket, point.average_release_year ?? 0));
  $: featurePoints = featureTimeline.map((point) =>
    metricPoint(point.bucket, point.average_features_per_song),
  );
  $: rangeSummary = buildRangeSummary(summary, releaseYears, featureAverage);
  $: bucketMetrics = [
    {
      key: "listens",
      label: "Listens",
      color: chartColor(0),
      points: listensPoints,
      valueLabel: "Listens",
      formatAxis: formatCountValue,
      formatTooltip: formatListensTooltip,
    },
    {
      key: "time",
      label: "Time listened",
      color: chartColor(1),
      points: timePoints,
      valueLabel: "Time listened",
      formatAxis: formatDurationValue,
      formatTooltip: formatDurationValue,
    },
    {
      key: "artists",
      label: "Different artists",
      color: chartColor(2),
      points: uniqueArtistPoints,
      valueLabel: "Artists",
      formatAxis: formatCountValue,
      formatTooltip: formatArtistsTooltip,
    },
  ];

  onMount(() => {
    stopReducedMotionWatch = watchReducedMotion();
    void initialize();

    unsubscribeRange = selectedStatsRange.subscribe((selection) => {
      const key = statsRangeSelectionKey(selection);
      if (key === lastRangeKey) return;
      lastRangeKey = key;
      void loadStats(selection);
    });

    return () => {
      unsubscribeRange?.();
      stopReducedMotionWatch?.();
      requestId += 1;
    };
  });

  async function initialize() {
    const selection = normalizeStatsRangeSelection(get(selectedStatsRange));
    lastRangeKey = statsRangeSelectionKey(selection);
    if (
      summary !== null &&
      statsRangeSelectionKey(selection) !== statsRangeSelectionKey(defaultRange)
    ) {
      clearDashboardData();
    }
    await loadStats(selection);
  }

  async function loadStats(range: StatsRangeSelection = activeRange) {
    const request = ++requestId;
    const initialLoad = summary === null;
    loading = initialLoad;
    refreshing = !initialLoad;
    if (initialLoad) {
      activeRange = range;
      fullDashboardLoaded = false;
    }
    error = null;

    try {
      const nextDashboard = await fetchDashboard(range);

      if (request !== requestId) return;
      applyDashboard(nextDashboard, range);
    } catch (err) {
      if (request !== requestId) return;
      error = err instanceof Error ? err.message : "Unable to load stats";
    } finally {
      if (request === requestId) {
        loading = false;
        refreshing = false;
      }
    }
  }

  function applyDashboard(
    nextDashboard: StatsDashboardResponse,
    range: StatsRangeSelection,
  ) {
    activeRange = range;
    availableYears = nextDashboard.available_years;
    summary = nextDashboard.summary;
    discovery = nextDashboard.discovery;
    artistDistributionRows = nextDashboard.artist_distribution;
    hours = nextDashboard.hours;
    hourlyArtists = nextDashboard.hourly_artists;
    timeline = nextDashboard.timeline;
    diversity = nextDashboard.diversity;
    releaseYears = nextDashboard.release_years;
    featureAverage = nextDashboard.feature_average;
    featureTimeline = nextDashboard.feature_timeline;
    sessions = nextDashboard.sessions;
    concentration = nextDashboard.concentration;
    comebackArtists = nextDashboard.comeback_artists;
    repeatLoops = nextDashboard.repeat_loops;
    bucketAxis = nextDashboard.bucket_axis.buckets;
    bucketAxisSplit = nextDashboard.bucket_axis.split;
    hourFormat = nextDashboard.hour_format;
    timezone = nextDashboard.timezone;
    fullDashboardLoaded = true;
  }

  function clearDashboardData() {
    summary = null;
    discovery = null;
    artistDistributionRows = [];
    hours = [];
    hourlyArtists = [];
    timeline = [];
    diversity = [];
    releaseYears = null;
    featureAverage = null;
    featureTimeline = [];
    sessions = null;
    concentration = null;
    comebackArtists = [];
    repeatLoops = null;
    bucketAxis = [];
    bucketAxisSplit = splitForStatsRange(activeRange.range);
    timezone = null;
    fullDashboardLoaded = false;
    refreshing = false;
  }

  function fetchDashboard(range: StatsRangeSelection): Promise<StatsDashboardResponse> {
    return apiFetch<StatsDashboardResponse>(dashboardPath(range));
  }

  function watchReducedMotion(): () => void {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => {
      prefersReducedMotion = query.matches;
    };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }

  function motionDuration(duration: number): number {
    return prefersReducedMotion ? 1 : duration;
  }

  function statsPath(
    path: string,
    query: string,
    extra: Record<string, string | number | boolean> = {},
  ): string {
    const params = new URLSearchParams(query);
    for (const [key, value] of Object.entries(extra))
      params.set(key, String(value));
    return `${apiPrefix}${path}?${params.toString()}`;
  }

  function dashboardPath(range: StatsRangeSelection): string {
    return statsPath("/stats/dashboard", statsRangeQuery(range), {
      split: splitForStatsRange(range.range),
    });
  }

  function splitLabel(split: TimeSplit): string {
    if (split === "hour") return "hourly";
    if (split === "day") return "daily";
    if (split === "week") return "weekly";
    if (split === "month") return "monthly";
    return "yearly";
  }

  function metricPoint(bucket: string, value: number): StatsMetricPoint {
    return {
      rawLabel: bucket,
      label: formatBucketLabel(bucket),
      value,
    };
  }

  function formatBucketLabel(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    if (timelineSplit === "year")
      return date.toLocaleDateString(undefined, { year: "numeric" });
    if (timelineSplit === "month")
      return date.toLocaleDateString(undefined, {
        month: "short",
        year: "2-digit",
      });
    if (timelineSplit === "week")
      return date.toLocaleDateString(undefined, {
        day: "numeric",
        month: "short",
      });
    if (timelineSplit === "hour") {
      const hour = formatHourLabel(date.getHours());
      if (activeRange.range === "today") return hour;
      const day = date.toLocaleDateString(undefined, {
        day: "numeric",
        weekday: "short",
      });
      return `${day}, ${hour}`;
    }
    return date.toLocaleDateString(undefined, {
      day: "numeric",
      month: "short",
    });
  }

  function bucketPreposition(): string {
    if (timelineSplit === "hour") return "at";
    if (timelineSplit === "day" || timelineSplit === "week") return "on";
    return "in";
  }

  function formatHourLabel(hour: number): string {
    if (hourFormat === "24") return `${String(hour).padStart(2, "0")}:00`;
    const suffix = hour < 12 ? "AM" : "PM";
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function formatReleaseYear(value: number | null | undefined): string {
    return typeof value === "number" && Number.isFinite(value)
      ? value.toFixed(1)
      : "n/a";
  }

  function formatFeatureAverage(value: number | null | undefined): string {
    return typeof value === "number" && Number.isFinite(value)
      ? value.toFixed(2)
      : "0.00";
  }

  function formatReleaseYearAxis(value: unknown): string {
    return Math.round(numericValue(value)).toString();
  }

  function formatReleaseYearValue(value: unknown): string {
    return formatReleaseYear(numericValue(value));
  }

  function formatFeatureValue(value: unknown): string {
    return numericValue(value).toLocaleString(undefined, {
      maximumFractionDigits: 2,
      minimumFractionDigits: 2,
    });
  }

  function formatListensTooltip(value: unknown): string {
    return `${formatCountValue(value)} listens`;
  }

  function formatArtistsTooltip(value: unknown): string {
    return `${formatCountValue(value)} artists`;
  }

  function buildRangeSummary(
    nextSummary: SummaryStats | null,
    nextReleaseYears: AlbumReleaseYearsStats | null,
    nextFeatureAverage: FeatureAverageStats | null,
  ): SummaryItem[] {
    if (!nextSummary) return [];
    return [
      {
        label: "Listens",
        value: nextSummary.total_listens.toLocaleString(),
        detail: activeRangeLabel,
      },
      {
        label: "Time listened",
        value: formatDuration(nextSummary.total_duration_ms),
        detail: activeRangeLabel,
      },
      {
        label: "Different artists",
        value: nextSummary.unique_artists.toLocaleString(),
        detail: activeRangeLabel,
      },
      {
        label: "Average release year",
        value: formatReleaseYear(nextReleaseYears?.average_release_year),
        detail: "weighted by listens",
      },
      {
        label: "Average features per track",
        value: formatFeatureAverage(
          nextFeatureAverage?.average_features_per_song,
        ),
        detail: `${nextFeatureAverage?.unique_tracks.toLocaleString() ?? "0"} tracks`,
      },
    ];
  }
</script>

<section class="stats-page page-stack" aria-busy={loading || refreshing}>
  <p class="sr-only" aria-live="polite">
    {loading ? "Loading stats" : refreshing ? "Refreshing stats" : "Stats loaded"}
  </p>
  <div class="stats-title">
    <div class="page-title">
      <p class="kicker">Archive stats</p>
      <h1>Stats</h1>
    </div>
    <StatsRangePicker {availableYears} ariaLabel="Choose stats range" />
  </div>

  {#if error && summary === null}
    <Card.Root>
      <Card.Content><p class="error">{error}</p></Card.Content>
    </Card.Root>
  {:else}
    {#if error}
      <p class="error" aria-live="polite">{error}</p>
    {/if}

    <SummaryRail items={rangeSummary} loading={summary === null} {motionDuration} />

    <div class="stats-grid">
      <LazyStatsPanel
        load={loadArtistDistributionChart}
        className="stats-card-motion span-8"
        skeletonKind="chart"
        index={1}
        loading={!fullDashboardLoaded}
        props={{
          rows: artistDistributionRows,
          bucketKeys,
          timelineDescription,
          pagePrefix,
          formatBucketLabel,
          formatBucketPreposition: bucketPreposition,
        }}
      />

      <LazyStatsPanel
        load={loadDiscoveryComfortCard}
        className="stats-card-motion span-4"
        skeletonKind="compact"
        index={2}
        loading={!fullDashboardLoaded}
        props={{ stats: discovery }}
      />

      <LazyStatsPanel
        load={loadDayDistributionChart}
        className="stats-card-motion span-8"
        skeletonKind="split"
        index={3}
        loading={!fullDashboardLoaded}
        props={{ points: hours, hourFormat }}
      />

      <LazyStatsPanel
        load={loadHourArtistHeatmap}
        className="stats-card-motion span-4"
        skeletonKind="heatmap"
        index={4}
        loading={!fullDashboardLoaded}
        props={{ artists: hourlyArtists, hours, hourFormat, pagePrefix }}
      />

      <LazyStatsPanel
        load={loadBucketMetricsChart}
        className="stats-card-motion span-12"
        skeletonKind="wide"
        index={5}
        loading={!fullDashboardLoaded}
        props={{
          title: "Bucket metrics",
          description: timelineDescription,
          metrics: bucketMetrics,
        }}
      />

      <LazyStatsPanel
        load={loadListeningSessionsPanel}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={6}
        loading={!fullDashboardLoaded}
        props={{ sessions, timezone }}
      />

      <LazyStatsPanel
        load={loadConcentrationPanel}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={7}
        loading={!fullDashboardLoaded}
        props={{ concentration, pagePrefix }}
      />

      <LazyStatsPanel
        load={loadComebackArtistsPanel}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={8}
        loading={!fullDashboardLoaded}
        props={{ artists: comebackArtists, pagePrefix, timezone }}
      />

      <LazyStatsPanel
        load={loadRepeatLoopsPanel}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={9}
        loading={!fullDashboardLoaded}
        props={{ loops: repeatLoops, pagePrefix }}
      />

      <LazyStatsPanel
        load={loadMetricChart}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={10}
        loading={!fullDashboardLoaded}
        props={{
          title: "Average release year",
          description: "listen-weighted by bucket",
          points: releaseYearPoints,
          valueLabel: "Average release year",
          color: chartColor(3),
          kind: "line",
          formatValue: formatReleaseYearValue,
          formatAxisValue: formatReleaseYearAxis,
          emptyLabel: "No release year data for this range.",
          zeroBased: false,
        }}
      />

      <LazyStatsPanel
        load={loadMetricChart}
        className="stats-card-motion span-6"
        skeletonKind="compact"
        index={11}
        loading={!fullDashboardLoaded}
        props={{
          title: "Average features per track",
          description: "distinct tracks by bucket",
          points: featurePoints,
          valueLabel: "Average features",
          color: chartColor(4),
          kind: "line",
          formatValue: formatFeatureValue,
          emptyLabel: "No feature data for this range.",
          zeroBased: false,
        }}
      />
    </div>
  {/if}
</section>

<style>
  .stats-title {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 0.1rem;
  }

  .stats-title h1 {
    font-size: clamp(1.7rem, 2.8vw, 2.18rem);
  }

  .stats-grid {
    display: grid;
    gap: 0.65rem;
  }

  .stats-grid {
    grid-template-columns: repeat(12, minmax(0, 1fr));
    align-items: stretch;
  }

  .stats-grid :global(.span-4) {
    grid-column: span 4;
  }
  .stats-grid :global(.span-6) {
    grid-column: span 6;
  }
  .stats-grid :global(.span-8) {
    grid-column: span 8;
  }
  .stats-grid :global(.span-12) {
    grid-column: span 12;
  }

  :global(.stats-card-motion) {
    min-width: 0;
    height: 100%;
  }

  :global(.stats-card),
  .stats-grid :global(.stats-metric-card),
  .stats-grid :global(.day-distribution-card),
  .stats-grid :global(.hour-artist-card) {
    min-width: 0;
    height: 100%;
    gap: 0.85rem;
    padding-block: 1rem;
  }

  .stats-grid :global([data-slot="card-header"]),
  .stats-grid :global([data-slot="card-content"]) {
    padding-inline: 1rem;
  }

  .error {
    margin: 0;
    color: var(--color-danger);
  }

  @media (max-width: 1180px) {
    .stats-grid :global(.span-4),
    .stats-grid :global(.span-6),
    .stats-grid :global(.span-8) {
      grid-column: span 12;
    }
  }

  @media (max-width: 680px) {
    .stats-title {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
