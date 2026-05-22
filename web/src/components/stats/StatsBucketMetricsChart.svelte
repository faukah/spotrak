<script lang="ts">
  import { scaleBand } from 'd3-scale';
  import { BarChart } from 'layerchart';
  import { quintOut } from 'svelte/easing';
  import { bandTickStride } from '../../lib/charts/ticks';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';
  import { Button } from '../ui/button';

  type StatsMetricPoint = {
    label: string;
    value: number;
    rawLabel?: string;
  };

  type BucketMetric = {
    key: string;
    label: string;
    color: string;
    points: StatsMetricPoint[];
    valueLabel: string;
    formatAxis: (value: unknown) => string;
    formatTooltip: (value: unknown) => string;
  };

  type BucketDatum = {
    label: string;
    rawLabel: string;
  } & Record<string, string | number>;

  type TooltipItem = { color?: string; key?: string };

  export let title = 'Bucket metrics';
  export let description = '';
  export let metrics: BucketMetric[] = [];
  export let emptyLabel = 'No bucket data for this range.';
  export let className = '';

  let hiddenMetricKeys = new Set<string>();
  let chartWidth = 960;

  $: visibleMetrics = metrics.filter((metric) => !hiddenMetricKeys.has(metric.key));
  $: hasPoints = metrics.some((metric) => metric.points.length > 0);
  $: metricsByKey = new Map(metrics.map((metric) => [metric.key, metric]));
  $: chartData = buildChartData(metrics);
  $: chartConfig = Object.fromEntries(
    metrics.map((metric) => [metric.key, { label: metric.label, color: metric.color }]),
  ) satisfies Chart.ChartConfig;
  $: series = visibleMetrics.map((metric) => ({
    key: metric.key,
    label: metric.label,
    value: metric.key,
    color: metric.color,
  }));
  $: xTickStride = bandTickStride(chartData.length, chartWidth, {
    minTickSpacing: 52,
    maxTicks: 14,
    minTicks: 4,
    horizontalPadding: 66,
  });
  $: chartHeight = chartWidth > 0 && chartWidth < 680 ? '18rem' : '22rem';

  function toggleMetric(key: string) {
    if (!hiddenMetricKeys.has(key) && visibleMetrics.length === 1) return;
    const next = new Set(hiddenMetricKeys);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    hiddenMetricKeys = next;
  }

  function buildChartData(inputMetrics: BucketMetric[]): BucketDatum[] {
    const buckets = new Map<string, BucketDatum>();
    for (const metric of inputMetrics) {
      for (const point of metric.points) {
        const rawLabel = point.rawLabel ?? point.label;
        const row = buckets.get(rawLabel) ?? { rawLabel, label: point.label };
        row[metric.key] = chartValue(metric, point.value);
        row[rawValueKey(metric.key)] = point.value;
        buckets.set(rawLabel, row);
      }
    }
    return [...buckets.values()];
  }

  function chartValue(metric: BucketMetric, value: number): number {
    if (metric.key === 'time') return value / 60_000;
    return value;
  }

  function rawValueKey(key: string): string {
    return `${key}Raw`;
  }

  function metricForKey(key: string | undefined): BucketMetric | undefined {
    if (!key) return undefined;
    return metricsByKey.get(key);
  }

  function metricValue(metric: BucketMetric, row: BucketDatum): number {
    const value = row[rawValueKey(metric.key)] ?? row[metric.key] ?? 0;
    return typeof value === 'number' && Number.isFinite(value) ? value : 0;
  }

  function tooltipValue(value: unknown, item: TooltipItem): string {
    const metric = metricForKey(item.key);
    if (!metric) return String(value ?? '');
    if (metric.key === 'time' && typeof value === 'number') return metric.formatTooltip(value * 60_000);
    return metric.formatTooltip(value);
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
  }

  function formatAxisValue(value: unknown): string {
    const numeric = typeof value === 'number' && Number.isFinite(value) ? value : Number(value);
    if (!Number.isFinite(numeric)) return '';
    return Intl.NumberFormat(undefined, {
      notation: Math.abs(numeric) >= 10_000 ? 'compact' : 'standard',
      maximumFractionDigits: numeric > 0 && numeric < 10 ? 1 : 0,
    }).format(numeric);
  }
</script>

<Card.Root class={`bucket-metrics-card ${className} overflow-visible`}>
  <Card.Header>
    <div>
      {#if description}<Card.Description>{description}</Card.Description>{/if}
      <Card.Title>{title}</Card.Title>
    </div>
    <div class="metric-toggles" aria-label="Visible bucket metrics">
      {#each metrics as metric (metric.key)}
        <Button
          variant={hiddenMetricKeys.has(metric.key) ? 'outline' : 'secondary'}
          size="xs"
          class="metric-toggle"
          aria-pressed={!hiddenMetricKeys.has(metric.key)}
          disabled={!hiddenMetricKeys.has(metric.key) && visibleMetrics.length === 1}
          onclick={() => toggleMetric(metric.key)}
        >
          {metric.label}
        </Button>
      {/each}
    </div>
  </Card.Header>
  <Card.Content>
    {#if !hasPoints}
      <p class="state">{emptyLabel}</p>
    {:else}
      <div class="bucket-chart-frame" bind:clientWidth={chartWidth}>
        <Chart.Container
          config={chartConfig}
          class="bucket-metric-chart"
          style={`height: ${chartHeight}; min-height: 0; aspect-ratio: auto;`}
          role="group"
          aria-label={`${title} chart`}
        >
          <BarChart
            data={chartData}
            xScale={scaleBand().padding(0.25)}
            x="label"
            {series}
            x1Scale={scaleBand().paddingInner(0.2)}
            seriesLayout="group"
            rule={false}
            grid={{ y: true }}
            bandPadding={0.2}
            padding={{ left: 54, right: 12, top: 12, bottom: 42 }}
            props={{
              bars: {
                stroke: 'none',
                strokeWidth: 0,
                rounded: 'all',
                motion: { type: 'tween', duration: 90, easing: quintOut },
              },
              xAxis: { ticks: xTickStride },
              yAxis: { format: formatAxisValue, ticks: 4 },
              tooltip: { context: { mode: 'band' } },
            }}
          >
            {#snippet tooltip()}
              <Chart.Tooltip contained="window" indicator="dashed">
                {#snippet formatter({ value, name, item })}
                  <div class="tooltip-row">
                    <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                    <span class="tooltip-name">{name}</span>
                    <span class="tooltip-value">{tooltipValue(value, item)}</span>
                  </div>
                {/snippet}
              </Chart.Tooltip>
            {/snippet}
          </BarChart>
        </Chart.Container>
      </div>
      <table class="sr-only">
        <caption>{title}</caption>
        <thead>
          <tr>
            <th scope="col">Time bucket</th>
            {#each metrics as metric (metric.key)}
              <th scope="col">{metric.valueLabel}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each chartData as row (row.rawLabel)}
            <tr>
              <td>{row.label}</td>
              {#each metrics as metric (metric.key)}
                <td>{metric.formatTooltip(metricValue(metric, row))}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.bucket-metrics-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.bucket-metrics-card [data-slot='card-header']) {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 1rem;
  }

  .metric-toggles {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    justify-content: flex-end;
  }

  .bucket-chart-frame {
    min-width: 0;
  }

  :global(.bucket-metric-chart) {
    width: 100%;
    min-width: 0;
    height: 22rem;
    aspect-ratio: auto;
  }

  .tooltip-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-width: 12rem;
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

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 680px) {
    :global(.bucket-metrics-card [data-slot='card-header']) {
      align-items: stretch;
      flex-direction: column;
    }

    .metric-toggles {
      justify-content: flex-start;
    }

    :global(.bucket-metric-chart) {
      height: 18rem;
    }
  }
</style>
