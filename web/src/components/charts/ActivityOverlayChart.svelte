<script lang="ts">
  import { onMount } from 'svelte';
  import { LineChart } from 'layerchart';
  import { apiFetch } from '../../lib/api/client';
  import type { TimelinePoint } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatDurationValue, formatLongDate, formatShortDate, numericValue, tickStep } from '../../lib/charts/theme';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type TooltipItem = { color?: string; key?: string };

  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';

  let points: TimelinePoint[] = [];
  let loading = true;
  let error: string | null = null;

  const chartConfig = {
    plays: { label: 'plays', color: chartColor(0) },
    minutes: { label: 'time', color: chartColor(1) },
  } satisfies Chart.ChartConfig;

  $: chartData = points.map((point) => ({
    label: point.bucket,
    plays: point.count,
    minutes: point.duration_ms / 60_000,
  }));
  $: xTickStep = tickStep(chartData.length, 6);

  const series = [
    { key: 'plays', label: chartConfig.plays.label, value: 'plays', color: chartConfig.plays.color },
    { key: 'minutes', label: chartConfig.minutes.label, value: 'minutes', color: chartConfig.minutes.color },
  ];

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = null;
    try {
      points = await apiFetch<TimelinePoint[]>(`/stats/listening-over-time?split=${split}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load activity';
    } finally {
      loading = false;
    }
  }

  function formatAxisValue(value: unknown): string {
    return formatCountValue(value);
  }

  function formatTooltipValue(value: unknown, item: TooltipItem): string {
    if (item.key === 'minutes') return formatDurationValue(numericValue(value) * 60_000);
    return `${formatCountValue(value)} plays`;
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
  }
</script>

<Card.Root class="activity-card">
  <Card.Header>
    <Card.Description>{split} buckets</Card.Description>
    <Card.Title>Activity overlay</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="chart skeleton"></div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if points.length === 0}
      <p class="state">No activity yet.</p>
    {:else}
      <Chart.Container config={chartConfig} class="chart" role="img" aria-label="Overlaid plays and listening time chart">
        <LineChart
          data={chartData}
          x="label"
          series={series}
          yBaseline={0}
          legend
          padding={{ left: 46, right: 24, top: 38, bottom: 42 }}
          props={{
            xAxis: { format: formatShortDate, ticks: xTickStep },
            yAxis: { format: formatAxisValue, tickSpacing: 72 },
            spline: { strokeWidth: 2.2 },
          }}
        >
          {#snippet tooltip()}
            <Chart.Tooltip labelFormatter={formatLongDate}>
              {#snippet formatter({ value, name, item })}
                <div class="tooltip-row">
                  <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                  <span class="tooltip-name">{name}</span>
                  <span class="tooltip-value">{formatTooltipValue(value, item)}</span>
                </div>
              {/snippet}
            </Chart.Tooltip>
          {/snippet}
        </LineChart>
      </Chart.Container>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.chart) {
    width: 100%;
    min-height: clamp(22rem, 43vw, 33rem);
  }

  .tooltip-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-width: 11rem;
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

  .state { color: var(--color-muted); }
  .error { color: var(--color-danger); }
</style>
