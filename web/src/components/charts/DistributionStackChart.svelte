<script lang="ts">
  import { onMount } from 'svelte';
  import { AreaChart } from 'layerchart';
  import { apiFetch } from '../../lib/api/client';
  import type { BucketedTopAlbum, BucketedTopArtist } from '../../lib/api/types';
  import { chartColor, formatLongDate, formatMetricValue, formatShortDate, tickStep } from '../../lib/charts/theme';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import { selectedStatsMetric } from '../../lib/stores/preferences';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';
  import * as Chart from '../ui/chart';

  export let kind: 'artists' | 'albums' = 'artists';
  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';
  export let limit = 8;

  type Row = BucketedTopArtist | BucketedTopAlbum;
  type Metric = 'count' | 'duration';
  type Entity = { id: string; name: string; image_url?: string | null; total: number };
  type ExpandedBucket = { bucket: string; total: number; values: Map<string, number> };
  type DistributionDatum = { bucket: string; total: number } & Record<string, string | number>;
  type TooltipItem = { color?: string };

  let rows: Row[] = [];
  let entities: Entity[] = [];
  let metric: Metric = 'count';
  let loading = true;
  let error: string | null = null;
  let unsubscribe: (() => void) | undefined;
  let loadRequest = 0;

  const metrics = [
    { value: 'count' as const, label: 'Plays' },
    { value: 'duration' as const, label: 'Time' },
  ];

  $: title = `${kind === 'artists' ? 'Artist' : 'Album'} distribution`;
  $: metricLabel = metric === 'duration' ? 'time' : 'plays';
  $: endpoint = `/stats/top/${kind}-by-bucket`;
  $: buckets = uniqueBuckets();
  $: chartData = buildChartData(buckets);
  $: chartConfig = Object.fromEntries(
    entities.map((entity, index) => [entity.id, { label: entity.name, color: color(index) }]),
  ) satisfies Chart.ChartConfig;
  $: series = entities.map((entity, index) => ({
    key: entity.id,
    label: entity.name,
    value: entity.id,
    color: color(index),
  }));
  $: xTickStep = tickStep(chartData.length, 6);

  onMount(() => {
    unsubscribe = selectedStatsMetric.subscribe((value) => {
      metric = value;
      void load();
    });
    return () => {
      unsubscribe?.();
      loadRequest += 1;
    };
  });

  async function load() {
    const request = ++loadRequest;
    loading = true;
    error = null;

    let nextRows: Row[] = [];
    let nextError: string | null = null;
    try {
      nextRows = await apiFetch<Row[]>(`${endpoint}?split=${split}&metric=${metric}&limit=${limit}`);
    } catch (err) {
      nextError = err instanceof Error ? err.message : `Unable to load ${kind} distribution`;
    }

    if (request !== loadRequest) return;
    rows = nextRows;
    error = nextError;
    entities = nextError ? [] : buildEntities(nextRows);
    loading = false;
  }

  function setMetric(value: Metric) {
    selectedStatsMetric.set(value);
  }

  function buildEntities(input: Row[]): Entity[] {
    const byId = new Map<string, Entity>();
    for (const row of input) {
      const value = metricValue(row);
      const current = byId.get(row.id) ?? { id: row.id, name: row.name, image_url: row.image_url, total: 0 };
      current.total += value;
      current.image_url ||= row.image_url;
      byId.set(row.id, current);
    }
    return [...byId.values()].toSorted((a, b) => b.total - a.total).slice(0, limit);
  }

  function uniqueBuckets(): string[] {
    return [...new Set(rows.map((row) => row.bucket))].toSorted();
  }

  function href(entity: Entity): string {
    return kind === 'artists' ? `/artist/${entity.id}` : `/album/${entity.id}`;
  }

  function coverTransition(entity: Entity, index: number): string {
    return viewTransitionName(entity.id, `distribution-${kind}-${split}-${limit}-${index}`);
  }

  function coverHref(entity: Entity, index: number): string {
    return transitionHref(href(entity), coverTransition(entity, index));
  }

  function color(index: number): string {
    return chartColor(index);
  }

  function metricValue(row: Row): number {
    return metric === 'duration' ? row.duration_ms : row.count;
  }

  function buildExpandedBuckets(inputBuckets: string[]): ExpandedBucket[] {
    const entityIds = new Set(entities.map((entity) => entity.id));
    const byBucket = new Map<string, Map<string, number>>();
    for (const bucket of inputBuckets) byBucket.set(bucket, new Map());

    for (const row of rows) {
      if (!entityIds.has(row.id)) continue;
      const values = byBucket.get(row.bucket);
      if (!values) continue;
      values.set(row.id, (values.get(row.id) ?? 0) + metricValue(row));
    }

    return inputBuckets.map((bucket) => {
      const values = byBucket.get(bucket) ?? new Map<string, number>();
      const total = [...values.values()].reduce((sum, value) => sum + value, 0);
      return { bucket, total, values };
    });
  }

  function buildChartData(inputBuckets: string[]): DistributionDatum[] {
    return buildExpandedBuckets(inputBuckets).map((bucket) => {
      const datum: DistributionDatum = { bucket: bucket.bucket, total: bucket.total };
      for (const entity of entities) {
        datum[entity.id] = bucket.values.get(entity.id) ?? 0;
      }
      return datum;
    });
  }

  function formatBucket(value: unknown): string {
    if (value === undefined || value === null) return '';
    const raw = String(value);
    const date = new Date(raw);
    if (Number.isNaN(date.getTime())) return raw;

    if (split === 'year') {
      return date.toLocaleDateString(undefined, { year: 'numeric' });
    }
    if (split === 'month') {
      return date.toLocaleDateString(undefined, { month: 'short' });
    }
    if (split === 'hour') {
      return date.toLocaleString(undefined, { day: 'numeric', hour: 'numeric', month: 'short' });
    }
    return formatShortDate(raw);
  }

  function formatTooltipLabel(value: unknown): string {
    if (split === 'day' || split === 'week') return formatLongDate(value);
    return formatBucket(value);
  }

  function formatRaw(value: unknown): string {
    const formatted = formatMetricValue(value, metric);
    return metric === 'count' ? `${formatted} plays` : formatted;
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
  }
</script>

<Card.Root class="distribution-card">
  <Card.Header class="distribution-header">
    <div>
      <Card.Description>top {limit} expanded by {metricLabel}</Card.Description>
      <Card.Title>{title}</Card.Title>
    </div>
    <div class="metric-buttons" aria-label="Distribution metric">
      {#each metrics as option (option.value)}
        <Button variant={metric === option.value ? 'default' : 'outline'} size="xs" onclick={() => setMetric(option.value)}>{option.label}</Button>
      {/each}
    </div>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="chart skeleton"></div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if rows.length === 0}
      <p class="state">Not enough data for distribution.</p>
    {:else}
      <div class="legend-strip" aria-label={`${kind} legend`}>
        {#each entities as entity, index (entity.id)}
          <a href={coverHref(entity, index)} title={entity.name} style={`--swatch: ${color(index)};`}>
            <CoverArt src={directImageUrl(entity)} name={entity.name} size="xs" transitionName={coverTransition(entity, index)} />
            <span>{entity.name}</span>
          </a>
        {/each}
      </div>
      <Chart.Container config={chartConfig} class="chart" role="img" aria-label={`${kind} listening distribution stacked expanded area chart`}>
        <AreaChart
          data={chartData}
          x="bucket"
          series={series}
          seriesLayout="stackExpand"
          padding={{ left: 42, right: 16, top: 14, bottom: 38 }}
          props={{
            xAxis: { format: formatBucket, ticks: xTickStep },
            area: { fillOpacity: 0.46 },
            line: { strokeWidth: 1.15 },
          }}
        >
          {#snippet tooltip()}
            <Chart.Tooltip labelFormatter={formatTooltipLabel}>
              {#snippet formatter({ value, name, item })}
                <div class="tooltip-row">
                  <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                  <span class="tooltip-name">{name}</span>
                  <span class="tooltip-value">{formatRaw(value)}</span>
                </div>
              {/snippet}
            </Chart.Tooltip>
          {/snippet}
        </AreaChart>
      </Chart.Container>
      <table class="sr-only">
        <caption>{title} data by bucket</caption>
        <thead>
          <tr>
            <th scope="col">Bucket</th>
            {#each entities as entity}<th scope="col">{entity.name}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each chartData as point}
            <tr>
              <td>{formatTooltipLabel(point.bucket)}</td>
              {#each entities as entity}<td>{formatRaw(point[entity.id])}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.distribution-header) {
    display: flex;
    flex-direction: row;
    align-items: start;
    justify-content: space-between;
    gap: 1rem;
  }

  .metric-buttons {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }

  .legend-strip {
    display: flex;
    gap: 0.35rem;
    overflow-x: auto;
    padding-bottom: 0.45rem;
    scrollbar-width: thin;
  }

  .legend-strip a {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    max-width: 12rem;
    border: 1px solid color-mix(in srgb, var(--swatch) 45%, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.45rem 0.25rem 0.25rem;
    background: color-mix(in srgb, var(--swatch) 13%, transparent);
    color: var(--color-text);
    font-size: 0.75rem;
    font-weight: 700;
    text-decoration: none;
  }

  .legend-strip span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.chart) {
    width: 100%;
    min-height: 22rem;
  }

  .tooltip-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-width: 12rem;
  }

  .tooltip-swatch {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 999px;
  }

  .tooltip-name {
    overflow: hidden;
    color: var(--color-text);
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip-value {
    color: var(--color-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .state {
    color: var(--color-muted);
  }
  .error {
    color: var(--color-danger);
  }

  @media (max-width: 620px) {
    :global(.distribution-header) {
      align-items: stretch;
      flex-direction: column;
    }

    .metric-buttons {
      flex-wrap: wrap;
    }
  }
</style>
