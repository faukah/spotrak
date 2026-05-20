<script lang="ts">
  import { onMount, tick } from 'svelte';
  import * as echarts from 'echarts/core';
  import { LineChart } from 'echarts/charts';
  import { GridComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { apiFetch } from '../../lib/api/client';
  import type { BucketedTopAlbum, BucketedTopArtist } from '../../lib/api/types';
  import { chartColors, chartTooltip, CHART_PALETTE } from '../../lib/charts/theme';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import { formatChartValue } from '../../lib/stats/format';
  import { selectedStatsMetric } from '../../lib/stores/preferences';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';

  echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer]);

  export let kind: 'artists' | 'albums' = 'artists';
  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';
  export let limit = 8;

  type Row = BucketedTopArtist | BucketedTopAlbum;
  type Metric = 'count' | 'duration';
  type Entity = { id: string; name: string; image_url?: string | null; total: number };
  type ExpandedBucket = { bucket: string; total: number; values: Map<string, number> };
  type PointData = { value: number; raw: number; total: number };
  type TooltipParam = {
    axisValue?: string | number;
    axisValueLabel?: string;
    color?: string;
    data?: unknown;
    seriesName?: string;
    value?: unknown;
  };

  let element: HTMLDivElement | null = null;
  let chart: echarts.ECharts | null = null;
  let rows: Row[] = [];
  let entities: Entity[] = [];
  let metric: Metric = 'count';
  let loading = true;
  let error: string | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let unsubscribe: (() => void) | undefined;
  let loadRequest = 0;

  const metrics = [
    { value: 'count' as const, label: 'Plays' },
    { value: 'duration' as const, label: 'Time' },
  ];

  $: title = `${kind === 'artists' ? 'Artist' : 'Album'} distribution`;
  $: metricLabel = metric === 'duration' ? 'time' : 'plays';
  $: endpoint = `/stats/top/${kind}-by-bucket`;

  onMount(() => {
    const handleThemeChange = () => render();
    resizeObserver = new ResizeObserver(() => chart?.resize());
    window.addEventListener('spotrak:theme-change', handleThemeChange);
    unsubscribe = selectedStatsMetric.subscribe((value) => {
      metric = value;
      void load();
    });
    return () => {
      window.removeEventListener('spotrak:theme-change', handleThemeChange);
      unsubscribe?.();
      loadRequest += 1;
      destroyChart();
    };
  });

  async function load() {
    const request = ++loadRequest;
    destroyChart();
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

    await tick();
    if (request !== loadRequest || error || rows.length === 0 || !element) return;
    chart = echarts.init(element);
    resizeObserver?.observe(element);
    render();
  }

  function destroyChart() {
    resizeObserver?.disconnect();
    chart?.dispose();
    chart = null;
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

  function uniqueBuckets() {
    return [...new Set(rows.map((row) => row.bucket))].toSorted();
  }

  function href(entity: Entity) {
    return kind === 'artists' ? `/artist/${entity.id}` : `/album/${entity.id}`;
  }

  function coverTransition(entity: Entity, index: number) {
    return viewTransitionName(entity.id, `distribution-${kind}-${split}-${limit}-${index}`);
  }

  function coverHref(entity: Entity, index: number) {
    return transitionHref(href(entity), coverTransition(entity, index));
  }

  function color(index: number) {
    return CHART_PALETTE[index % CHART_PALETTE.length];
  }

  function metricValue(row: Row) {
    return metric === 'duration' ? row.duration_ms : row.count;
  }

  function buildExpandedBuckets(buckets: string[]): ExpandedBucket[] {
    const entityIds = new Set(entities.map((entity) => entity.id));
    const byBucket = new Map<string, Map<string, number>>();
    for (const bucket of buckets) byBucket.set(bucket, new Map());

    for (const row of rows) {
      if (!entityIds.has(row.id)) continue;
      const values = byBucket.get(row.bucket);
      if (!values) continue;
      values.set(row.id, (values.get(row.id) ?? 0) + metricValue(row));
    }

    return buckets.map((bucket) => {
      const values = byBucket.get(bucket) ?? new Map<string, number>();
      const total = [...values.values()].reduce((sum, value) => sum + value, 0);
      return { bucket, total, values };
    });
  }

  function render() {
    if (!chart) return;
    const colors = chartColors();
    const buckets = uniqueBuckets();
    const expandedBuckets = buildExpandedBuckets(buckets);

    const series = entities.map((entity, index) => ({
      name: entity.name,
      type: 'line',
      stack: 'expanded-distribution',
      smooth: 0.35,
      symbol: 'none',
      showSymbol: false,
      lineStyle: { width: 1.15, color: color(index), opacity: 0.92 },
      areaStyle: { color: color(index), opacity: 0.46 },
      emphasis: { focus: 'series', lineStyle: { width: 2 } },
      data: expandedBuckets.map((bucket) => {
        const raw = bucket.values.get(entity.id) ?? 0;
        const value = bucket.total > 0 ? (raw / bucket.total) * 100 : 0;
        return { value, raw, total: bucket.total } satisfies PointData;
      }),
    }));

    chart.setOption(
      {
        backgroundColor: 'transparent',
        animationDuration: 550,
        color: entities.map((_, index) => color(index)),
        tooltip: {
          trigger: 'axis',
          axisPointer: {
            type: 'line',
            lineStyle: { color: colors.muted, type: 'dashed', width: 1 },
          },
          ...chartTooltip(colors),
          formatter: tooltipFormatter,
        },
        grid: { left: 42, right: 16, top: 14, bottom: 38 },
        xAxis: {
          type: 'category',
          data: buckets,
          boundaryGap: false,
          axisLabel: { color: colors.muted, formatter: axisLabel, hideOverlap: true, fontSize: 11 },
          axisLine: { lineStyle: { color: colors.border } },
          axisTick: { show: false },
        },
        yAxis: {
          type: 'value',
          min: 0,
          max: 100,
          interval: 25,
          splitNumber: 4,
          axisLabel: { color: colors.muted, formatter: (value: number) => `${value}%` },
          splitLine: { lineStyle: { color: colors.border, opacity: 0.38 } },
        },
        series,
      },
      true,
    );
  }

  function axisLabel(value: string) {
    return formatBucket(value, false);
  }

  function formatBucket(value: string | number | undefined, long: boolean) {
    if (value === undefined) return '';
    const raw = String(value);
    const date = new Date(raw);
    if (Number.isNaN(date.getTime())) return raw;

    if (split === 'year') {
      return date.toLocaleDateString(undefined, { year: 'numeric' });
    }
    if (split === 'month') {
      return date.toLocaleDateString(undefined, { month: 'short', year: long ? 'numeric' : undefined });
    }
    if (split === 'hour') {
      return date.toLocaleString(undefined, {
        day: long ? 'numeric' : undefined,
        hour: 'numeric',
        month: long ? 'short' : undefined,
      });
    }
    return date.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: long ? 'numeric' : undefined });
  }

  function tooltipFormatter(params: unknown) {
    const items = tooltipItems(params)
      .map((item) => ({ item, point: pointFromParam(item) }))
      .filter(({ point }) => point.raw > 0)
      .toReversed();

    const first = tooltipItems(params)[0];
    const label = formatBucket(first?.axisValueLabel ?? first?.axisValue, true);
    const total = items[0]?.point.total ?? 0;
    const rowsHtml = items.map(({ item, point }) => tooltipRow(item, point)).join('');
    const emptyText = `No plays in the selected ${kind}.`;

    return `
      <div class="distribution-tooltip">
        <div class="tooltip-title">${escapeHtml(label)}</div>
        <div class="tooltip-total">Shown total: ${escapeHtml(formatRaw(total))}</div>
        ${rowsHtml || `<div class="tooltip-total">${emptyText}</div>`}
      </div>
    `;
  }

  function tooltipRow(item: TooltipParam, point: PointData) {
    const name = escapeHtml(item.seriesName ?? 'Unknown');
    const colorValue = typeof item.color === 'string' ? item.color : 'currentColor';
    return `
      <div class="tooltip-row">
        <span class="tooltip-swatch" style="background: ${colorValue}"></span>
        <span class="tooltip-name">${name}</span>
        <span class="tooltip-value">${formatPercent(point.value)} · ${escapeHtml(formatRaw(point.raw))}</span>
      </div>
    `;
  }

  function tooltipItems(params: unknown): TooltipParam[] {
    const values = Array.isArray(params) ? params : [params];
    return values.filter(isTooltipParam);
  }

  function isTooltipParam(value: unknown): value is TooltipParam {
    return typeof value === 'object' && value !== null;
  }

  function pointFromParam(param: TooltipParam): PointData {
    if (isPointData(param.data)) return param.data;
    const value = typeof param.value === 'number' ? param.value : 0;
    return { value, raw: 0, total: 0 };
  }

  function isPointData(value: unknown): value is PointData {
    if (typeof value !== 'object' || value === null) return false;
    const point = value as Partial<PointData>;
    return typeof point.value === 'number' && typeof point.raw === 'number' && typeof point.total === 'number';
  }

  function formatPercent(value: number) {
    return `${value.toLocaleString(undefined, { maximumFractionDigits: 1, minimumFractionDigits: value > 0 && value < 1 ? 1 : 0 })}%`;
  }

  function formatRaw(value: number) {
    const formatted = formatChartValue(value, metric);
    return metric === 'count' ? `${formatted} plays` : formatted;
  }

  function escapeHtml(value: string) {
    const replacements: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    };
    return value.replace(/[&<>"']/g, (char) => replacements[char] ?? char);
  }
</script>

<Card.Root class="distribution-card">
  <Card.Header class="distribution-header">
    <div>
      <Card.Description>top {limit} expanded by {metricLabel}</Card.Description>
      <Card.Title>{title}</Card.Title>
    </div>
    <div class="metric-buttons" aria-label="Distribution metric">
      {#each metrics as option}
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
        {#each entities as entity, index}
          <a href={coverHref(entity, index)} title={entity.name} style={`--swatch: ${color(index)};`}>
            <CoverArt src={directImageUrl(entity)} name={entity.name} size="xs" transitionName={coverTransition(entity, index)} />
            <span>{entity.name}</span>
          </a>
        {/each}
      </div>
      <div bind:this={element} class="chart" role="img" aria-label={`${kind} listening distribution stacked expanded area chart`}></div>
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

  :global(.distribution-tooltip) {
    display: grid;
    gap: 0.35rem;
    min-width: 12rem;
  }

  :global(.distribution-tooltip .tooltip-title) {
    color: var(--color-text);
    font-weight: 800;
  }

  :global(.distribution-tooltip .tooltip-total) {
    color: var(--color-muted);
    font-size: 0.78rem;
  }

  :global(.distribution-tooltip .tooltip-row) {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
  }

  :global(.distribution-tooltip .tooltip-swatch) {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 999px;
  }

  :global(.distribution-tooltip .tooltip-name) {
    overflow: hidden;
    color: var(--color-text);
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.distribution-tooltip .tooltip-value) {
    color: var(--color-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
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

  .chart {
    width: 100%;
    min-height: 22rem;
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
