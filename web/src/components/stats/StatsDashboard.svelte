<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { apiFetch } from '../../lib/api/client';
  import type {
    AlbumReleaseYearsStats,
    BucketedTopArtist,
    DiversityTimelinePoint,
    FeatureAverageStats,
    FeatureTimelinePoint,
    HourRepartitionPoint,
    HourlyTopArtist,
    StatsDashboardResponse,
    StatsRangeKey,
    SummaryStats,
    TimelinePoint,
    TopArtist,
  } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatDurationValue, numericValue } from '../../lib/charts/theme';
  import { formatDuration } from '../../lib/date/format';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import {
    selectedStatsRange,
    statsRangeLabel,
    statsRangeQuery,
    statsRangeSelectionKey,
    type StatsRangeSelection,
  } from '../../lib/stores/stats-range';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import StatsArtistDistributionChart from './StatsArtistDistributionChart.svelte';
  import StatsBucketMetricsChart from './StatsBucketMetricsChart.svelte';
  import StatsDayDistributionChart from './StatsDayDistributionChart.svelte';
  import StatsHourArtistHeatmap from './StatsHourArtistHeatmap.svelte';
  import StatsMetricChart from './StatsMetricChart.svelte';
  import StatsRangePicker from './StatsRangePicker.svelte';

  type TimeSplit = 'year' | 'month' | 'week' | 'day' | 'hour';
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

  export let apiPrefix = '';
  export let pagePrefix = '';
  export let initialHourFormat: '12' | '24' = '24';

  const currentYear = new Date().getFullYear();

  let activeRange: StatsRangeSelection = { range: 'all' };
  let availableYears: number[] = [currentYear];
  let summary: SummaryStats | null = null;
  let topArtists: TopArtist[] = [];
  let artistDistributionRows: BucketedTopArtist[] = [];
  let hours: HourRepartitionPoint[] = [];
  let hourlyArtists: HourlyTopArtist[] = [];
  let timeline: TimelinePoint[] = [];
  let diversity: DiversityTimelinePoint[] = [];
  let releaseYears: AlbumReleaseYearsStats | null = null;
  let featureAverage: FeatureAverageStats | null = null;
  let featureTimeline: FeatureTimelinePoint[] = [];
  let hourFormat: '12' | '24' = initialHourFormat;
  let loading = true;
  let refreshing = false;
  let error: string | null = null;
  let requestId = 0;
  let unsubscribeRange: (() => void) | undefined;
  let lastRangeKey = statsRangeSelectionKey(activeRange);

  $: timelineSplit = splitForRange(activeRange.range);
  $: timelineDescription = `${splitLabel(timelineSplit)} buckets`;
  $: activeRangeLabel = statsRangeLabel(activeRange);
  $: topArtistMax = Math.max(1, ...topArtists.map((artist) => artist.count));
  $: bucketKeys = buildBucketKeys(activeRange, timelineSplit, [
    ...artistDistributionRows.map((row) => row.bucket),
    ...timeline.map((point) => point.bucket),
    ...diversity.map((point) => point.bucket),
    ...featureTimeline.map((point) => point.bucket),
  ]);
  $: timelineByBucket = new Map(timeline.map((point) => [point.bucket, point]));
  $: diversityByBucket = new Map(diversity.map((point) => [point.bucket, point]));
  $: listensPoints = bucketKeys.map((bucket) => metricPoint(bucket, timelineByBucket.get(bucket)?.count ?? 0));
  $: timePoints = bucketKeys.map((bucket) => metricPoint(bucket, timelineByBucket.get(bucket)?.duration_ms ?? 0));
  $: uniqueArtistPoints = bucketKeys.map((bucket) => metricPoint(bucket, diversityByBucket.get(bucket)?.unique_artists ?? 0));
  $: releaseYearPoints = diversity
    .filter((point) => point.average_release_year !== null && point.average_release_year !== undefined)
    .map((point) => metricPoint(point.bucket, point.average_release_year ?? 0));
  $: featurePoints = featureTimeline.map((point) => metricPoint(point.bucket, point.average_features_per_song));
  $: rangeSummary = buildRangeSummary(summary, releaseYears, featureAverage);
  $: bucketMetrics = [
    {
      key: 'listens',
      label: 'Listens',
      color: chartColor(0),
      points: listensPoints,
      valueLabel: 'Listens',
      formatAxis: formatCountValue,
      formatTooltip: formatListensTooltip,
    },
    {
      key: 'time',
      label: 'Time listened',
      color: chartColor(1),
      points: timePoints,
      valueLabel: 'Time listened',
      formatAxis: formatDurationValue,
      formatTooltip: formatDurationValue,
    },
    {
      key: 'artists',
      label: 'Different artists',
      color: chartColor(2),
      points: uniqueArtistPoints,
      valueLabel: 'Artists',
      formatAxis: formatCountValue,
      formatTooltip: formatArtistsTooltip,
    },
  ];

  onMount(() => {
    activeRange = get(selectedStatsRange);
    lastRangeKey = statsRangeSelectionKey(activeRange);
    void loadStats();

    unsubscribeRange = selectedStatsRange.subscribe((selection) => {
      const key = statsRangeSelectionKey(selection);
      if (key === lastRangeKey) return;
      activeRange = selection;
      lastRangeKey = key;
      void loadStats();
    });

    return () => {
      unsubscribeRange?.();
      requestId += 1;
    };
  });

  async function loadStats() {
    const request = ++requestId;
    const range = activeRange;
    const split = splitForRange(range.range);
    const query = statsRangeQuery(range);
    loading = summary === null;
    refreshing = summary !== null;
    error = null;

    try {
      const nextDashboard = await apiFetch<StatsDashboardResponse>(statsPath('/stats/dashboard', query, { split }));

      if (request !== requestId) return;
      availableYears = nextDashboard.available_years;
      summary = nextDashboard.summary;
      topArtists = nextDashboard.top_artists;
      artistDistributionRows = nextDashboard.artist_distribution;
      hours = nextDashboard.hours;
      hourlyArtists = nextDashboard.hourly_artists;
      timeline = nextDashboard.timeline;
      diversity = nextDashboard.diversity;
      releaseYears = nextDashboard.release_years;
      featureAverage = nextDashboard.feature_average;
      featureTimeline = nextDashboard.feature_timeline;
      hourFormat = nextDashboard.hour_format;
    } catch (err) {
      if (request !== requestId) return;
      error = err instanceof Error ? err.message : 'Unable to load stats';
    } finally {
      if (request === requestId) {
        loading = false;
        refreshing = false;
      }
    }
  }

  function statsPath(path: string, query: string, extra: Record<string, string | number | boolean> = {}): string {
    const params = new URLSearchParams(query);
    for (const [key, value] of Object.entries(extra)) params.set(key, String(value));
    return `${apiPrefix}${path}?${params.toString()}`;
  }

  function splitForRange(range: StatsRangeKey): TimeSplit {
    if (range === 'today') return 'hour';
    if (range === 'week' || range === 'month') return 'day';
    if (range === 'year') return 'week';
    return 'month';
  }

  function splitLabel(split: TimeSplit): string {
    if (split === 'hour') return 'hourly';
    if (split === 'day') return 'daily';
    if (split === 'week') return 'weekly';
    if (split === 'month') return 'monthly';
    return 'yearly';
  }

  function buildBucketKeys(selection: StatsRangeSelection, split: TimeSplit, rawBuckets: string[]): string[] {
    const rawDates = rawBuckets
      .map(parseBucketDate)
      .filter((date): date is Date => date !== null);
    const now = new Date();
    const selectedYear = selection.year ?? currentYear;
    let start: Date | null = null;
    let end: Date | null = null;

    if (selection.range === 'today') {
      start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      end = startOfBucket(now, split);
    } else if (selection.range === 'week') {
      start = startOfBucket(now, 'week');
      end = startOfBucket(now, split);
    } else if (selection.range === 'month') {
      start = new Date(now.getFullYear(), now.getMonth(), 1);
      end = startOfBucket(now, split);
    } else if (selection.range === 'year') {
      start = startOfBucket(new Date(now.getFullYear(), 0, 1), split);
      end = startOfBucket(now, split);
    } else if (selection.range === 'selected-year') {
      start = startOfBucket(new Date(selectedYear, 0, 1), split);
      end = startOfBucket(new Date(selectedYear, 11, 31, 23), split);
    } else if (rawDates.length > 0) {
      const sorted = rawDates.toSorted((a, b) => a.getTime() - b.getTime());
      start = startOfBucket(sorted[0], split);
      end = startOfBucket(sorted[sorted.length - 1], split);
    }

    if (!start || !end || start > end) return rawBuckets.toSorted();

    const buckets: string[] = [];
    for (let cursor = new Date(start); cursor <= end; cursor = addBucket(cursor, split)) {
      buckets.push(formatBucketKey(cursor));
      if (buckets.length > 1200) break;
    }
    return buckets;
  }

  function parseBucketDate(value: string): Date | null {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  function startOfBucket(date: Date, split: TimeSplit): Date {
    const next = new Date(date);
    next.setMinutes(0, 0, 0);
    if (split === 'hour') return next;
    next.setHours(0, 0, 0, 0);
    if (split === 'day') return next;
    if (split === 'week') {
      const mondayOffset = (next.getDay() + 6) % 7;
      next.setDate(next.getDate() - mondayOffset);
      return next;
    }
    next.setDate(1);
    if (split === 'month') return next;
    next.setMonth(0, 1);
    return next;
  }

  function addBucket(date: Date, split: TimeSplit): Date {
    const next = new Date(date);
    if (split === 'hour') next.setHours(next.getHours() + 1);
    else if (split === 'day') next.setDate(next.getDate() + 1);
    else if (split === 'week') next.setDate(next.getDate() + 7);
    else if (split === 'month') next.setMonth(next.getMonth() + 1);
    else next.setFullYear(next.getFullYear() + 1);
    return next;
  }

  function formatBucketKey(date: Date): string {
    return `${date.getFullYear()}-${padDatePart(date.getMonth() + 1)}-${padDatePart(date.getDate())}T${padDatePart(date.getHours())}:00:00`;
  }

  function padDatePart(value: number): string {
    return String(value).padStart(2, '0');
  }

  function metricPoint(bucket: string, value: number): StatsMetricPoint {
    return {
      rawLabel: bucket,
      label: formatBucketLabel(bucket),
      value,
    };
  }

  function artistHref(id: string): string {
    return `${pagePrefix}/artist/${id}`;
  }

  function artistTransition(id: string, scope: string): string {
    return viewTransitionName(id, scope);
  }

  function artistTransitionHref(id: string, scope: string): string {
    return transitionHref(artistHref(id), artistTransition(id, scope));
  }

  function barPercent(value: number, max: number): string {
    return `${Math.max(4, Math.min(100, (value / Math.max(1, max)) * 100))}%`;
  }

  function formatBucketLabel(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    if (timelineSplit === 'year') return date.toLocaleDateString(undefined, { year: 'numeric' });
    if (timelineSplit === 'month') return date.toLocaleDateString(undefined, { month: 'short', year: '2-digit' });
    if (timelineSplit === 'week') return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
    if (timelineSplit === 'hour') {
      const hour = formatHourLabel(date.getHours());
      if (activeRange.range === 'today') return hour;
      const day = date.toLocaleDateString(undefined, { day: 'numeric', weekday: 'short' });
      return `${day}, ${hour}`;
    }
    return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  }

  function formatHourLabel(hour: number): string {
    if (hourFormat === '24') return `${String(hour).padStart(2, '0')}:00`;
    const suffix = hour < 12 ? 'AM' : 'PM';
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function formatReleaseYear(value: number | null | undefined): string {
    return typeof value === 'number' && Number.isFinite(value) ? value.toFixed(1) : 'n/a';
  }

  function formatFeatureAverage(value: number | null | undefined): string {
    return typeof value === 'number' && Number.isFinite(value) ? value.toFixed(2) : '0.00';
  }

  function formatReleaseYearAxis(value: unknown): string {
    return Math.round(numericValue(value)).toString();
  }

  function formatReleaseYearValue(value: unknown): string {
    return formatReleaseYear(numericValue(value));
  }

  function formatFeatureValue(value: unknown): string {
    return numericValue(value).toLocaleString(undefined, { maximumFractionDigits: 2, minimumFractionDigits: 2 });
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
      { label: 'Listens', value: nextSummary.total_listens.toLocaleString(), detail: activeRangeLabel },
      { label: 'Time listened', value: formatDuration(nextSummary.total_duration_ms), detail: activeRangeLabel },
      { label: 'Different artists', value: nextSummary.unique_artists.toLocaleString(), detail: activeRangeLabel },
      { label: 'Average release year', value: formatReleaseYear(nextReleaseYears?.average_release_year), detail: 'weighted by listens' },
      { label: 'Average features per track', value: formatFeatureAverage(nextFeatureAverage?.average_features_per_song), detail: `${nextFeatureAverage?.unique_tracks.toLocaleString() ?? '0'} tracks` },
    ];
  }

</script>

<section class="stats-page page-stack" aria-busy={refreshing}>
  <div class="stats-title">
    <div class="page-title">
      <p class="kicker">Archive stats</p>
      <h1>Stats</h1>
      <p>{activeRangeLabel}, {timelineDescription}. Distribution, timing, volume, diversity, release era, and feature density.</p>
    </div>
    <StatsRangePicker {availableYears} ariaLabel="Choose stats range" />
  </div>

  {#if loading}
    <div class="stats-skeleton-grid" aria-live="polite">
      {#each Array(8) as _, index (index)}
        <div class="skeleton stats-skeleton"></div>
      {/each}
    </div>
  {:else if error}
    <Card.Root>
      <Card.Content><p class="error">{error}</p></Card.Content>
    </Card.Root>
  {:else}
    {#if refreshing}<p class="refresh-note" aria-live="polite">Refreshing {activeRangeLabel}…</p>{/if}

    <section class="stat-rail" aria-label="Selected range summary">
      {#each rangeSummary as item (item.label)}
        <article>
          <span>{item.label}</span>
          <strong>{item.value}</strong>
          <small>{item.detail}</small>
        </article>
      {/each}
    </section>

    <div class="stats-grid">
      <StatsArtistDistributionChart
        className="span-8"
        rows={artistDistributionRows}
        {bucketKeys}
        {timelineDescription}
        {pagePrefix}
        formatBucketLabel={formatBucketLabel}
      />

      <Card.Root class="stats-card span-4 top-artists-card">
        <Card.Header>
          <Card.Description>ranked by listens</Card.Description>
          <Card.Title>Top artists</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if topArtists.length === 0}
            <p class="state">No artist data for this range.</p>
          {:else}
            <ol class="rank-bars">
              {#each topArtists as artist, index (artist.id)}
                <li style={`--bar: ${barPercent(artist.count, topArtistMax)}; --swatch: ${chartColor(index)};`}>
                  <span class="rank">{String(index + 1).padStart(2, '0')}</span>
                  <CoverArt src={directImageUrl(artist)} name={artist.name} href={artistTransitionHref(artist.id, `stats-top-artists-${index}`)} size="xs" transitionName={artistTransition(artist.id, `stats-top-artists-${index}`)} />
                  <a class="artist-name" href={artistHref(artist.id)} title={artist.name}>
                    <strong>{artist.name}</strong>
                    <small>{formatListensTooltip(artist.count)}</small>
                  </a>
                  <span class="rank-track" aria-hidden="true"><span></span></span>
                </li>
              {/each}
            </ol>
            <table class="sr-only">
              <caption>Top artists by listens</caption>
              <thead><tr><th scope="col">Artist</th><th scope="col">Listens</th></tr></thead>
              <tbody>
                {#each topArtists as artist (artist.id)}
                  <tr><td>{artist.name}</td><td>{artist.count}</td></tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </Card.Content>
      </Card.Root>

      <StatsDayDistributionChart points={hours} {hourFormat} className="span-8" />
      <StatsHourArtistHeatmap artists={hourlyArtists} {hours} {hourFormat} {pagePrefix} className="span-4" />

      <StatsBucketMetricsChart className="span-12" title="Bucket metrics" description={timelineDescription} metrics={bucketMetrics} />
      <StatsMetricChart className="span-6" title="Average release year" description="listen-weighted by bucket" points={releaseYearPoints} valueLabel="Average release year" color={chartColor(3)} kind="line" formatValue={formatReleaseYearValue} formatAxisValue={formatReleaseYearAxis} emptyLabel="No release year data for this range." zeroBased={false} />
      <StatsMetricChart className="span-6" title="Average features per track" description="distinct tracks by bucket" points={featurePoints} valueLabel="Average features" color={chartColor(4)} kind="line" formatValue={formatFeatureValue} emptyLabel="No feature data for this range." zeroBased={false} />
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

  .refresh-note {
    margin: -0.25rem 0;
    color: var(--color-muted);
    font-size: 0.82rem;
  }

  .stats-skeleton-grid,
  .stat-rail,
  .stats-grid {
    display: grid;
    gap: 0.65rem;
  }

  .stats-skeleton-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .stats-skeleton {
    min-height: 14rem;
    border-radius: var(--radius-lg);
  }

  .stat-rail {
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-bg-elevated) 82%, transparent);
  }

  .stat-rail article {
    display: grid;
    gap: 0.14rem;
    min-width: 0;
    border-right: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    padding: 0.62rem 0.75rem;
  }

  .stat-rail article:last-child {
    border-right: 0;
  }

  .stat-rail span,
  .stat-rail small {
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .stat-rail small {
    overflow: hidden;
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-rail strong {
    font-size: clamp(1.02rem, 1.45vw, 1.34rem);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.045em;
    line-height: 1;
  }

  .stats-grid {
    grid-template-columns: repeat(12, minmax(0, 1fr));
    align-items: stretch;
  }

  .stats-grid :global(.span-4) { grid-column: span 4; }
  .stats-grid :global(.span-6) { grid-column: span 6; }
  .stats-grid :global(.span-8) { grid-column: span 8; }
  .stats-grid :global(.span-12) { grid-column: span 12; }

  :global(.stats-card),
  .stats-grid :global(.stats-metric-card),
  .stats-grid :global(.day-distribution-card),
  .stats-grid :global(.hour-artist-card) {
    min-width: 0;
    height: 100%;
    gap: 0.85rem;
    padding-block: 1rem;
  }

  .stats-grid :global([data-slot='card-header']),
  .stats-grid :global([data-slot='card-content']) {
    padding-inline: 1rem;
  }

  .rank-bars {
    display: grid;
    gap: 0.42rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rank-bars li {
    display: grid;
    grid-template-columns: 1.55rem auto minmax(0, 1fr) minmax(4rem, 7rem);
    gap: 0.45rem;
    align-items: center;
  }

  .rank {
    color: color-mix(in srgb, var(--color-muted) 70%, transparent);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    font-weight: 800;
  }

  .artist-name {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
    color: var(--color-text);
    text-decoration: none;
  }

  .artist-name strong,
  .artist-name small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist-name strong {
    font-size: 0.82rem;
    line-height: 1.08;
  }

  .artist-name small {
    color: var(--color-muted);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
  }

  .rank-track {
    grid-column: 4;
    display: block;
    height: 0.24rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 54%, transparent);
  }

  .rank-track span {
    display: block;
    width: var(--bar);
    height: 100%;
    border-radius: inherit;
    background: var(--swatch);
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
  }

  @media (max-width: 1180px) {
    .stat-rail,
    .stats-skeleton-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .stat-rail article {
      border-right: 0;
      border-bottom: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    }

    .stat-rail article:nth-child(odd) {
      border-right: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    }

    .stat-rail article:last-child,
    .stat-rail article:nth-last-child(2):nth-child(odd) {
      border-bottom: 0;
    }

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

    .stats-skeleton-grid {
      grid-template-columns: 1fr;
    }

    .stat-rail {
      grid-template-columns: repeat(2, minmax(0, 1fr)) !important;
    }

    .stat-rail article {
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: baseline;
    }

    .stat-rail article:last-child {
      grid-column: 1 / -1;
    }

    .stat-rail small {
      grid-column: 1 / -1;
    }

    .rank-bars li {
      grid-template-columns: 1.45rem auto minmax(0, 1fr) 4.5rem !important;
    }

    .rank-track {
      grid-column: 4 !important;
    }
  }
</style>
