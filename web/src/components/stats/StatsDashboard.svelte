<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { scaleUtc } from 'd3-scale';
  import { curveMonotoneX } from 'd3-shape';
  import { AreaChart } from 'layerchart';
  import { apiFetch } from '../../lib/api/client';
  import type {
    AlbumReleaseYearsStats,
    BucketedTopArtist,
    DiversityTimelinePoint,
    FeatureAverageStats,
    FeatureTimelinePoint,
    HourRepartitionPoint,
    HourlyTopArtist,
    StatsRangeKey,
    SummaryStats,
    TimelinePoint,
    TopArtist,
  } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatDurationValue, formatPercentValue, numericValue, tickStep } from '../../lib/charts/theme';
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
  import * as Chart from '../ui/chart';
  import StatsHourDistribution from './StatsHourDistribution.svelte';
  import StatsLineChart from './StatsLineChart.svelte';
  import StatsRangePicker from './StatsRangePicker.svelte';

  type TimeSplit = 'year' | 'month' | 'week' | 'day' | 'hour';
  type DistributionEntity = {
    id: string;
    name: string;
    image_url?: string | null;
    total: number;
    isOther: boolean;
  };
  type DistributionSegment = DistributionEntity & {
    value: number;
    percent: number;
    color: string;
  };
  type DistributionBucket = {
    bucket: string;
    total: number;
    segments: DistributionSegment[];
  };
  type HourArtistSlot = {
    hour: number;
    label: string;
    artist: HourlyTopArtist | null;
    percent: number;
  };
  type DistributionDatum = {
    bucket: string;
    date: Date;
    total: number;
  } & Record<string, Date | string | number>;
  type TooltipItem = { color?: string };
  type TooltipPayloadValue = { value?: unknown };

  export let apiPrefix = '';
  export let pagePrefix = '';

  const currentYear = new Date().getFullYear();
  const DISTRIBUTION_ARTIST_LIMIT = 10;
  const OTHER_ARTISTS_ID = '__other__';

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
  ]);
  $: timelineByBucket = new Map(timeline.map((point) => [point.bucket, point]));
  $: diversityByBucket = new Map(diversity.map((point) => [point.bucket, point]));
  $: distributionEntities = buildDistributionEntities(artistDistributionRows);
  $: distributionBuckets = buildDistributionBuckets(artistDistributionRows, distributionEntities, bucketKeys);
  $: distributionChartBuckets = distributionBuckets.filter((bucket) => bucket.total > 0);
  $: distributionChartData = buildDistributionChartData(distributionChartBuckets, distributionEntities);
  $: distributionChartConfig = Object.fromEntries(
    distributionEntities.map((entity, index) => [entity.id, { label: entity.name, color: distributionColor(index, entity) }]),
  ) satisfies Chart.ChartConfig;
  $: distributionSeries = distributionEntities.map((entity, index) => ({
    key: entity.id,
    label: entity.name,
    value: entity.id,
    color: distributionColor(index, entity),
    props: distributionAreaProps(entity),
  }));
  $: areaChartWidth = Math.min(5600, Math.max(860, distributionChartData.length * areaPointWidth(timelineSplit)));
  $: distributionTickCount = Math.min(8, Math.max(2, distributionChartData.length));
  $: hourArtistSlots = buildHourArtistSlots(hourlyArtists);
  $: songPoints = bucketKeys.map((bucket) => ({ label: bucket, value: timelineByBucket.get(bucket)?.count ?? 0 }));
  $: timePoints = bucketKeys.map((bucket) => ({ label: bucket, value: timelineByBucket.get(bucket)?.duration_ms ?? 0 }));
  $: uniqueArtistPoints = bucketKeys.map((bucket) => ({ label: bucket, value: diversityByBucket.get(bucket)?.unique_artists ?? 0 }));
  $: releaseYearPoints = diversity
    .filter((point) => point.average_release_year !== null && point.average_release_year !== undefined)
    .map((point) => ({ label: point.bucket, value: point.average_release_year ?? 0 }));
  $: featurePoints = featureTimeline.map((point) => ({ label: point.bucket, value: point.average_features_per_song }));
  $: rangeSummary = summary
    ? [
        { label: 'songs', value: summary.total_listens.toLocaleString(), detail: activeRangeLabel },
        { label: 'time', value: formatDuration(summary.total_duration_ms), detail: activeRangeLabel },
        { label: 'artists', value: summary.unique_artists.toLocaleString(), detail: activeRangeLabel },
        { label: 'avg release', value: formatReleaseYear(releaseYears?.average_release_year), detail: 'weighted by plays' },
        { label: 'avg feats', value: formatFeatureAverage(featureAverage?.average_features_per_song), detail: `${featureAverage?.featured_tracks.toLocaleString() ?? '0'} featured songs` },
      ]
    : [];

  onMount(() => {
    activeRange = get(selectedStatsRange);
    lastRangeKey = statsRangeSelectionKey(activeRange);
    void loadAvailableYears();
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
      const [
        nextSummary,
        nextTopArtists,
        nextArtistDistribution,
        nextHours,
        nextHourlyArtists,
        nextTimeline,
        nextDiversity,
        nextReleaseYears,
        nextFeatureAverage,
        nextFeatureTimeline,
      ] = await Promise.all([
        apiFetch<SummaryStats>(statsPath('/stats/summary', query)),
        apiFetch<TopArtist[]>(statsPath('/stats/top/artists', query, { limit: 12, metric: 'count' })),
        apiFetch<BucketedTopArtist[]>(statsPath('/stats/top/artists-by-bucket', query, { split, metric: 'count', limit: DISTRIBUTION_ARTIST_LIMIT, group_other: 'true' })),
        apiFetch<HourRepartitionPoint[]>(statsPath('/stats/hour-repartition/tracks', query)),
        apiFetch<HourlyTopArtist[]>(statsPath('/stats/top/artists-by-hour', query, { limit: 1, metric: 'count' })),
        apiFetch<TimelinePoint[]>(statsPath('/stats/listening-over-time', query, { split })),
        apiFetch<DiversityTimelinePoint[]>(statsPath('/stats/diversity-over-time', query, { split })),
        apiFetch<AlbumReleaseYearsStats>(statsPath('/stats/album-release-years', query)),
        apiFetch<FeatureAverageStats>(statsPath('/stats/feature-average', query)),
        apiFetch<FeatureTimelinePoint[]>(statsPath('/stats/feature-average-over-time', query, { split })),
      ]);

      if (request !== requestId) return;
      summary = nextSummary;
      topArtists = nextTopArtists;
      artistDistributionRows = nextArtistDistribution;
      hours = nextHours;
      hourlyArtists = nextHourlyArtists;
      timeline = nextTimeline;
      diversity = nextDiversity;
      releaseYears = nextReleaseYears;
      featureAverage = nextFeatureAverage;
      featureTimeline = nextFeatureTimeline;
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

  async function loadAvailableYears() {
    try {
      const points = await apiFetch<TimelinePoint[]>(`${apiPrefix}/stats/listening-over-time?split=year`);
      const years = points
        .map((point) => Number(point.bucket.slice(0, 4)))
        .filter((year) => Number.isInteger(year));
      availableYears = Array.from(new Set([currentYear, ...years])).toSorted((a, b) => b - a);
    } catch {
      availableYears = [currentYear];
    }
  }

  function statsPath(path: string, query: string, extra: Record<string, string | number> = {}): string {
    const params = new URLSearchParams(query);
    for (const [key, value] of Object.entries(extra)) params.set(key, String(value));
    return `${apiPrefix}${path}?${params.toString()}`;
  }

  function splitForRange(range: StatsRangeKey): TimeSplit {
    if (range === 'today' || range === 'week') return 'hour';
    if (range === 'month') return 'day';
    return 'week';
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

  function buildDistributionEntities(rows: BucketedTopArtist[]): DistributionEntity[] {
    const byId = new Map<string, DistributionEntity>();
    for (const row of rows) {
      const isOther = row.id === OTHER_ARTISTS_ID;
      const current = byId.get(row.id) ?? { id: row.id, name: row.name, image_url: row.image_url, total: 0, isOther };
      current.total += row.count;
      current.image_url ||= row.image_url;
      current.isOther ||= isOther;
      byId.set(row.id, current);
    }
    return [...byId.values()].toSorted((a, b) => {
      if (a.isOther !== b.isOther) return a.isOther ? 1 : -1;
      return b.total - a.total;
    });
  }

  function buildDistributionBuckets(rows: BucketedTopArtist[], entities: DistributionEntity[], buckets: string[]): DistributionBucket[] {
    const entityIds = new Set(entities.map((entity) => entity.id));
    const byBucket = new Map<string, Map<string, number>>();
    for (const row of rows) {
      if (!entityIds.has(row.id)) continue;
      const bucket = byBucket.get(row.bucket) ?? new Map<string, number>();
      bucket.set(row.id, (bucket.get(row.id) ?? 0) + row.count);
      byBucket.set(row.bucket, bucket);
    }

    return buckets.map((bucket) => {
      const values = byBucket.get(bucket) ?? new Map<string, number>();
      const total = [...values.values()].reduce((sum, value) => sum + value, 0);
      const segments = entities.map((entity, index) => {
        const value = values.get(entity.id) ?? 0;
        return {
          ...entity,
          value,
          percent: total > 0 ? (value / total) * 100 : 0,
          color: distributionColor(index, entity),
        };
      });
      return { bucket, total, segments };
    });
  }

  function buildDistributionChartData(buckets: DistributionBucket[], entities: DistributionEntity[]): DistributionDatum[] {
    return buckets.map((bucket) => {
      const date = parseBucketDate(bucket.bucket) ?? new Date(bucket.bucket);
      const datum: DistributionDatum = { bucket: bucket.bucket, date, total: bucket.total };
      for (const entity of entities) datum[entity.id] = distributionSegmentValue(bucket, entity.id);
      return datum;
    });
  }

  function buildHourArtistSlots(rows: HourlyTopArtist[]): HourArtistSlot[] {
    const byHour = new Map(rows.map((artist) => [artist.hour, artist]));
    const max = Math.max(1, ...rows.map((artist) => artist.count));
    return Array.from({ length: 24 }, (_, hour) => {
      const artist = byHour.get(hour) ?? null;
      return {
        hour,
        label: formatHour(hour),
        artist,
        percent: artist ? (artist.count / max) * 100 : 0,
      };
    });
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
      const hour = date.toLocaleTimeString(undefined, { hour: '2-digit' });
      if (activeRange.range === 'today') return hour;
      const day = date.toLocaleDateString(undefined, { day: 'numeric', weekday: 'short' });
      return `${day}, ${hour}`;
    }
    return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  }

  function formatHour(hour: number): string {
    return `${String(hour).padStart(2, '0')}:00`;
  }

  function formatReleaseYear(value: number | null | undefined): string {
    return typeof value === 'number' && Number.isFinite(value) ? value.toFixed(1) : 'n/a';
  }

  function formatFeatureAverage(value: number | null | undefined): string {
    return typeof value === 'number' && Number.isFinite(value) ? value.toFixed(2) : '0.00';
  }

  function formatReleaseYearValue(value: unknown): string {
    return formatReleaseYear(numericValue(value));
  }

  function formatFeatureValue(value: unknown): string {
    return numericValue(value).toLocaleString(undefined, { maximumFractionDigits: 2, minimumFractionDigits: 2 });
  }

  function formatPlays(value: unknown): string {
    return `${formatCountValue(value)} plays`;
  }

  function formatDistributionValue(value: number): string {
    return `${value.toLocaleString()} plays`;
  }

  function areaPointWidth(split: TimeSplit): number {
    if (split === 'hour') return 14;
    if (split === 'day') return 24;
    if (split === 'week') return 18;
    return 34;
  }

  function formatDistributionTooltipLabel(value: unknown): string {
    if (value instanceof Date) return formatBucketDateLabel(value);
    return formatBucketLabel(String(value ?? ''));
  }

  function formatBucketDateLabel(date: Date): string {
    if (timelineSplit === 'year') return date.toLocaleDateString(undefined, { year: 'numeric' });
    if (timelineSplit === 'month') return date.toLocaleDateString(undefined, { month: 'short', year: '2-digit' });
    if (timelineSplit === 'week') return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
    if (timelineSplit === 'hour') {
      const hour = date.toLocaleTimeString(undefined, { hour: '2-digit' });
      if (activeRange.range === 'today') return hour;
      const day = date.toLocaleDateString(undefined, { day: 'numeric', weekday: 'short' });
      return `${day}, ${hour}`;
    }
    return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  }

  function formatDistributionTooltipValue(value: unknown, payload: TooltipPayloadValue[]): string {
    const raw = numericValue(value);
    const total = payload.reduce((sum, item) => sum + numericValue(item.value), 0);
    const share = total > 0 ? (raw / total) * 100 : 0;
    return `${formatCountValue(raw)} plays · ${formatPercentValue(share)}`;
  }

  function formatDistributionCell(bucket: DistributionBucket, entityId: string): string {
    const segment = bucket.segments.find((item) => item.id === entityId);
    const value = segment?.value ?? 0;
    const percent = segment?.percent ?? 0;
    return `${formatDistributionValue(value)} · ${formatPercentValue(percent)}`;
  }

  function distributionSegmentValue(bucket: DistributionBucket, entityId: string): number {
    return bucket.segments.find((segment) => segment.id === entityId)?.value ?? 0;
  }

  function distributionColor(index: number, entity?: DistributionEntity): string {
    return entity?.isOther ? 'var(--color-muted)' : chartColor(index);
  }

  function distributionAreaProps(entity: DistributionEntity) {
    if (entity.isOther) {
      return {
        role: 'presentation',
        tabindex: -1,
        'aria-hidden': true,
      };
    }

    return {
      onclick: () => openArtistFromDistribution(entity.id),
      onkeydown: (event: KeyboardEvent) => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        event.preventDefault();
        openArtistFromDistribution(entity.id);
      },
      role: 'link',
      tabindex: 0,
      'aria-label': `Open artist ${entity.name}`,
    };
  }

  function openArtistFromDistribution(id: string) {
    window.location.assign(artistHref(id));
  }

  function hasDistributionTooltipValue(item: TooltipPayloadValue): boolean {
    return numericValue(item.value) > 0;
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
  }
</script>

<section class="stats-page page-stack">
  <div class="stats-title">
    <div class="page-title">
      <p class="kicker">Archive stats</p>
      <h1>Stats</h1>
      <p>{activeRangeLabel}, {timelineDescription}. Rankings, timing, diversity, release era, and featured-artist density.</p>
    </div>
    <StatsRangePicker {availableYears} ariaLabel="Choose stats time range" />
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
      <Card.Root class="stats-card span-4 top-artists-card">
        <Card.Header>
          <Card.Description>ranked by plays</Card.Description>
          <Card.Title>Best artists by songs listened</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if topArtists.length === 0}
            <p class="state">No artist data for this range.</p>
          {:else}
            <ol class="rank-bars">
              {#each topArtists as artist, index (artist.id)}
                <li style={`--bar: ${barPercent(artist.count, topArtistMax)}; --swatch: ${chartColor(index)};`}>
                  <span class="rank">{String(index + 1).padStart(2, '0')}</span>
                  <CoverArt src={directImageUrl(artist)} name={artist.name} href={artistTransitionHref(artist.id, `stats-best-artists-${index}`)} size="xs" transitionName={artistTransition(artist.id, `stats-best-artists-${index}`)} />
                  <a class="artist-name" href={artistHref(artist.id)}>
                    <strong>{artist.name}</strong>
                    <small>{formatPlays(artist.count)}</small>
                  </a>
                  <span class="rank-track" aria-hidden="true"><span></span></span>
                </li>
              {/each}
            </ol>
            <table class="sr-only">
              <caption>Best artists by songs listened</caption>
              <thead><tr><th scope="col">Artist</th><th scope="col">Plays</th></tr></thead>
              <tbody>
                {#each topArtists as artist (artist.id)}
                  <tr><td>{artist.name}</td><td>{artist.count}</td></tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root class="stats-card span-8 artist-distribution-card">
        <Card.Header>
          <Card.Description>top {DISTRIBUTION_ARTIST_LIMIT} plus Other by {timelineDescription}</Card.Description>
          <Card.Title>Artist listening distribution</Card.Title>
        </Card.Header>
        <Card.Content class="artist-distribution-content">
          {#if distributionChartBuckets.length === 0}
            <p class="state">Not enough artist distribution data for this range.</p>
          {:else}
            <div class="distribution-legend" aria-label="Artist distribution legend">
              {#each distributionEntities as entity, index (entity.id)}
                {#if entity.isOther}
                  <span class="legend-other" style={`--swatch: ${distributionColor(index, entity)};`} title={`${entity.name}: ${formatDistributionValue(entity.total)}`}>
                    <span class="legend-swatch" aria-hidden="true"></span>
                    <span>{entity.name}</span>
                  </span>
                {:else}
                  <a href={artistTransitionHref(entity.id, `stats-distribution-${index}`)} style={`--swatch: ${distributionColor(index, entity)};`}>
                    <CoverArt src={directImageUrl(entity)} name={entity.name} size="xs" transitionName={artistTransition(entity.id, `stats-distribution-${index}`)} />
                    <span>{entity.name}</span>
                  </a>
                {/if}
              {/each}
            </div>
            <div class="stacked-area-scroll">
              <Chart.Container
                config={distributionChartConfig}
                class="stacked-area-chart"
                style={`--area-width: ${areaChartWidth}px;`}
                role="group"
                aria-label={`Top ${DISTRIBUTION_ARTIST_LIMIT} artist share by ${timelineDescription}, with the rest grouped as Other. Artist areas open detail pages.`}
              >
                <AreaChart
                  data={distributionChartData}
                  x="date"
                  xScale={scaleUtc()}
                  series={distributionSeries}
                  seriesLayout="stackExpand"
                  padding={{ left: 44, right: 18, top: 14, bottom: 38 }}
                  props={{
                    xAxis: { format: formatDistributionTooltipLabel, ticks: distributionTickCount },
                    area: {
                      curve: curveMonotoneX,
                      fillOpacity: 0.5,
                      line: { strokeWidth: 1.1, role: 'presentation', tabindex: -1, 'aria-hidden': true, 'aria-label': undefined },
                      motion: 'tween',
                    },
                    tooltip: { context: { mode: 'quadtree-x' } },
                  }}
                >
                  {#snippet tooltip()}
                    <Chart.Tooltip class="distribution-tooltip" labelFormatter={formatDistributionTooltipLabel} filter={hasDistributionTooltipValue} indicator="line">
                      {#snippet formatter({ value, name, item, payload })}
                        <div class="tooltip-row">
                          <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                          <span class="tooltip-name">{name}</span>
                          <span class="tooltip-value">{formatDistributionTooltipValue(value, payload)}</span>
                        </div>
                      {/snippet}
                    </Chart.Tooltip>
                  {/snippet}
                </AreaChart>
              </Chart.Container>
            </div>
            <table class="sr-only">
              <caption>Artist listening distribution data</caption>
              <thead>
                <tr>
                  <th scope="col">Bucket</th>
                  <th scope="col">Artist</th>
                  <th scope="col">Share</th>
                </tr>
              </thead>
              <tbody>
                {#each distributionChartBuckets as bucket (bucket.bucket)}
                  {#each bucket.segments.filter((segment) => segment.value > 0) as segment (segment.id)}
                    <tr>
                      <td>{formatBucketLabel(bucket.bucket)}</td>
                      <td>{segment.name}</td>
                      <td>{formatDistributionCell(bucket, segment.id)}</td>
                    </tr>
                  {/each}
                {/each}
              </tbody>
            </table>
          {/if}
        </Card.Content>
      </Card.Root>

      <StatsHourDistribution points={hours} className="span-8" />

      <Card.Root class="stats-card span-4 hour-artists-card">
        <Card.Header>
          <Card.Description>best artist per local hour</Card.Description>
          <Card.Title>Best artists for hour of day</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if hourlyArtists.length === 0}
            <p class="state">No hourly artist data for this range.</p>
          {:else}
            <div class="hour-artist-list">
              {#each hourArtistSlots as slot (slot.hour)}
                {#if slot.artist}
                  <a class="hour-artist-row" href={artistHref(slot.artist.artist_id)} style={`--bar: ${slot.percent}%;`}>
                    <span class="hour-label">{slot.label}</span>
                    <span class="hour-artist-name">{slot.artist.artist_name}</span>
                    <span class="hour-count">{slot.artist.count.toLocaleString()}</span>
                    <span class="hour-bar" aria-hidden="true"><span></span></span>
                  </a>
                {:else}
                  <div class="hour-artist-row empty" style="--bar: 0%;">
                    <span class="hour-label">{slot.label}</span>
                    <span class="hour-artist-name">No plays</span>
                    <span class="hour-count">0</span>
                    <span class="hour-bar" aria-hidden="true"><span></span></span>
                  </div>
                {/if}
              {/each}
            </div>
            <table class="sr-only">
              <caption>Best artists by local hour</caption>
              <thead><tr><th scope="col">Hour</th><th scope="col">Artist</th><th scope="col">Plays</th></tr></thead>
              <tbody>
                {#each hourArtistSlots as slot (slot.hour)}
                  <tr><td>{slot.label}</td><td>{slot.artist?.artist_name ?? 'No plays'}</td><td>{slot.artist?.count ?? 0}</td></tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </Card.Content>
      </Card.Root>

      <StatsLineChart className="span-6" title="Songs listened" description={timelineDescription} points={songPoints} valueLabel="Songs" color={chartColor(0)} formatValue={formatPlays} formatLabel={formatBucketLabel} />
      <StatsLineChart className="span-6" title="Time listened" description={timelineDescription} points={timePoints} valueLabel="Time" color={chartColor(1)} formatValue={formatDurationValue} formatLabel={formatBucketLabel} />
      <StatsLineChart className="span-6" title="Different artists listened" description={timelineDescription} points={uniqueArtistPoints} valueLabel="Artists" color={chartColor(2)} formatValue={formatCountValue} formatLabel={formatBucketLabel} />
      <StatsLineChart className="span-6" title="Average album release date" description="average release year" points={releaseYearPoints} valueLabel="Average release year" color={chartColor(3)} formatValue={formatReleaseYearValue} formatLabel={formatBucketLabel} emptyLabel="No release year data for this range." zeroBased={false} />
      <StatsLineChart className="span-6" title="Average feats per song" description="featured artists on listened songs" points={featurePoints} valueLabel="Average features" color={chartColor(4)} formatValue={formatFeatureValue} formatLabel={formatBucketLabel} emptyLabel="No feature data for this range." zeroBased={false} />
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
    font-size: clamp(1.08rem, 1.7vw, 1.45rem);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.055em;
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
  :global(.stats-line-card),
  :global(.hour-distribution-card) {
    min-width: 0;
    height: 100%;
    gap: 0.85rem;
    padding-block: 1rem;
  }

  .stats-grid :global([data-slot="card-header"]),
  .stats-grid :global([data-slot="card-content"]) {
    padding-inline: 1rem;
  }

  .rank-bars {
    display: grid;
    gap: 0.34rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rank-bars li {
    display: grid;
    grid-template-columns: 1.55rem auto minmax(0, 1fr) minmax(4rem, 7.5rem);
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
    height: 0.22rem;
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

  :global(.artist-distribution-content) {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
  }

  .distribution-legend {
    display: flex;
    gap: 0.3rem;
    overflow-x: auto;
    padding-bottom: 0.35rem;
    scrollbar-width: thin;
  }

  .distribution-legend a,
  .distribution-legend .legend-other {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    max-width: 11rem;
    border: 1px solid color-mix(in srgb, var(--swatch) 44%, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.45rem 0.25rem 0.25rem;
    background: color-mix(in srgb, var(--swatch) 12%, transparent);
    color: var(--color-text);
    font-size: 0.7rem;
    font-weight: 700;
    text-decoration: none;
  }

  .distribution-legend .legend-other {
    color: var(--color-muted);
  }

  .legend-swatch {
    width: 1.35rem;
    height: 1.35rem;
    flex: 0 0 auto;
    border: 1px solid color-mix(in srgb, var(--swatch) 52%, var(--color-border));
    border-radius: var(--radius-xs);
    background: color-mix(in srgb, var(--swatch) 72%, transparent);
  }

  .distribution-legend a > span:last-child,
  .distribution-legend .legend-other > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stacked-area-scroll {
    flex: 1;
    min-height: 0;
    overflow-x: auto;
    padding: 0.2rem 0 0.1rem;
    scrollbar-width: thin;
  }

  :global(.stacked-area-chart) {
    width: var(--area-width);
    min-width: 100%;
    min-height: 19.5rem;
  }

  :global(.stacked-area-chart .lc-area-path) {
    cursor: pointer;
    opacity: 0.84;
  }

  :global(.stacked-area-chart .lc-area-path[aria-hidden="true"]) {
    cursor: default;
  }

  :global(.stacked-area-chart .lc-area-path:focus-visible) {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 70%, transparent);
    outline-offset: 2px;
  }

  :global(.stacked-area-chart .lc-spline-path) {
    pointer-events: none;
    stroke-linejoin: round;
  }

  :global(.distribution-tooltip) {
    max-height: min(20rem, calc(100vh - 2rem));
    overflow: auto;
  }

  .tooltip-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-width: 13rem;
  }

  .tooltip-swatch {
    width: 0.58rem;
    height: 0.58rem;
    border-radius: 999px;
  }

  .tooltip-name {
    overflow: hidden;
    color: var(--color-text);
    font-weight: 750;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip-value {
    color: var(--color-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .hour-artist-list {
    display: grid;
    gap: 0.28rem;
    max-height: 17rem;
    overflow: auto;
    padding-right: 0.15rem;
  }

  .hour-artist-row {
    display: grid;
    grid-template-columns: 3.15rem minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 0.38rem 0.42rem;
    color: var(--color-text);
    text-decoration: none;
  }

  a.hour-artist-row:hover,
  a.hour-artist-row:focus-visible {
    border-color: color-mix(in srgb, var(--color-primary) 42%, var(--color-border));
    background: color-mix(in srgb, var(--color-panel-2) 52%, transparent);
    outline: none;
  }

  .hour-artist-row.empty {
    color: var(--color-muted);
  }

  .hour-label,
  .hour-count {
    color: var(--color-muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    font-weight: 800;
  }

  .hour-artist-name {
    overflow: hidden;
    font-size: 0.78rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hour-bar {
    grid-column: 2 / 4;
    display: block;
    height: 0.22rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 54%, transparent);
  }

  .hour-bar span {
    display: block;
    width: var(--bar);
    height: 100%;
    border-radius: inherit;
    background: var(--chart-1);
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
