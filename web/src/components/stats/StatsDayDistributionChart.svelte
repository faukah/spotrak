<script lang="ts">
  import { onMount } from 'svelte';
  import { BarChart } from 'layerchart';
  import type { HourRepartitionPoint } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatDurationValue } from '../../lib/charts/theme';
  import { bandTickStride } from '../../lib/charts/ticks';
  import { formatDuration } from '../../lib/date/format';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';
  import { Button } from '../ui/button';

  type MetricKey = 'listens' | 'time';
  type TooltipItem = { color?: string };

  export let points: HourRepartitionPoint[] = [];
  export let hourFormat: '12' | '24' = '24';
  export let className = '';

  let visibleMetrics: Record<MetricKey, boolean> = {
    listens: true,
    time: true,
  };
  let mounted = false;
  let chartWidth = 720;

  onMount(() => {
    mounted = true;
  });

  $: hourData = Array.from({ length: 24 }, (_, hour) => {
    const point = points.find((item) => item.hour === hour);
    return {
      hour,
      label: formatHour(hour),
      listens: point?.count ?? 0,
      time: point?.duration_ms ?? 0,
    };
  });
  $: visibleRows = metricRows.filter((row) => visibleMetrics[row.key]);
  $: chartHeight = visibleRows.length === 1 ? '18rem' : 'clamp(9.5rem, 22vw, 10.5rem)';
  $: hourTickStride = bandTickStride(24, chartWidth, {
    minTickSpacing: hourFormat === '12' ? 34 : 29,
    maxTicks: 24,
    minTicks: 4,
    horizontalPadding: 66,
  });
  $: hasData = points.length > 0;

  const metricRows = [
    {
      key: 'listens' as const,
      label: 'Listens',
      color: chartColor(0),
      format: formatCountValue,
      tooltipFormat: (value: unknown) => `${formatCountValue(value)} listens`,
    },
    {
      key: 'time' as const,
      label: 'Time listened',
      color: chartColor(1),
      format: formatDurationValue,
      tooltipFormat: formatDurationValue,
    },
  ];

  function formatHour(hour: number): string {
    if (hourFormat === '24') return `${String(hour).padStart(2, '0')}:00`;
    const suffix = hour < 12 ? 'AM' : 'PM';
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function toggleMetric(metric: MetricKey) {
    if (visibleMetrics[metric] && visibleRows.length === 1) return;
    visibleMetrics = {
      ...visibleMetrics,
      [metric]: !visibleMetrics[metric],
    };
  }

  function metricData(key: MetricKey) {
    return hourData.map((point) => ({
      label: point.label,
      value: point[key],
      hour: point.hour,
    }));
  }

  function metricConfig(key: MetricKey, color: string): Chart.ChartConfig {
    return {
      [key]: { label: metricLabel(key), color },
    };
  }

  function metricLabel(key: MetricKey): string {
    return key === 'listens' ? 'Listens' : 'Time listened';
  }

  function metricSeries(key: MetricKey, color: string) {
    return [{ key, label: metricLabel(key), value: 'value', color }];
  }

  function tooltipColor(item: TooltipItem, fallback: string): string {
    return item.color ?? fallback;
  }
</script>

<Card.Root class={`day-distribution-card ${className} overflow-visible`}>
  <Card.Header>
    <div>
      <Card.Description>{hourFormat === '24' ? '24-hour local time' : '12-hour local time'}</Card.Description>
      <Card.Title>Listening distribution over day</Card.Title>
    </div>
    <div class="metric-toggles" aria-label="Visible day distribution metrics">
      {#each metricRows as metric (metric.key)}
        <Button
          variant={visibleMetrics[metric.key] ? 'secondary' : 'outline'}
          size="xs"
          class="metric-toggle"
          aria-pressed={visibleMetrics[metric.key]}
          disabled={visibleMetrics[metric.key] && visibleRows.length === 1}
          onclick={() => toggleMetric(metric.key)}
        >
          <span aria-hidden="true">{visibleMetrics[metric.key] ? '✓' : ''}</span>
          {metric.label}
        </Button>
      {/each}
    </div>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No hourly listening data for this range.</p>
    {:else if !mounted}
      <div class="skeleton day-chart-loading" aria-hidden="true"></div>
    {:else}
      <div class="day-chart-stack" class:single-row={visibleRows.length === 1} bind:clientWidth={chartWidth}>
        {#each visibleRows as metric (metric.key)}
          <section class="day-chart-row" aria-label={`${metric.label} by local hour`}>
            <div class="row-heading">
              <span class="row-swatch" style:background={metric.color}></span>
              <strong>{metric.label}</strong>
            </div>
            <Chart.Container
              config={metricConfig(metric.key, metric.color)}
              class="day-metric-chart"
              style={`height: ${chartHeight}; min-height: 0; aspect-ratio: auto;`}
              role="group"
              aria-label={`${metric.label} by local hour`}
            >
              <BarChart
                data={metricData(metric.key)}
                x="label"
                y="value"
                series={metricSeries(metric.key, metric.color)}
                yDomain={[0, null]}
                bandPadding={0.28}
                padding={{ left: 52, right: 14, top: 8, bottom: 34 }}
                grid={{ y: true }}
                props={{
                  xAxis: { ticks: hourTickStride },
                  yAxis: { format: metric.format, ticks: 3 },
                  bars: { radius: 4, strokeWidth: 0 },
                  tooltip: { context: { mode: 'band' } },
                }}
              >
                {#snippet tooltip()}
                  <Chart.Tooltip contained="window" labelFormatter={(value) => String(value)} indicator="line">
                    {#snippet formatter({ value, name, item })}
                      <div class="tooltip-row">
                        <span class="tooltip-swatch" style:background={tooltipColor(item, metric.color)}></span>
                        <span class="tooltip-name">{name}</span>
                        <span class="tooltip-value">{metric.tooltipFormat(value)}</span>
                      </div>
                    {/snippet}
                  </Chart.Tooltip>
                {/snippet}
              </BarChart>
            </Chart.Container>
          </section>
        {/each}
      </div>
      <table class="sr-only">
        <caption>Listening distribution over day</caption>
        <thead>
          <tr><th scope="col">Hour</th><th scope="col">Listens</th><th scope="col">Time listened</th></tr>
        </thead>
        <tbody>
          {#each hourData as point (point.hour)}
            <tr><td>{point.label}</td><td>{point.listens}</td><td>{formatDuration(point.time)}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.day-distribution-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.day-distribution-card [data-slot='card-header']) {
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

  :global(.metric-toggle) {
    min-width: 0;
  }

  :global(.metric-toggle span) {
    display: inline-block;
    width: 0.65rem;
    color: var(--color-primary);
    text-align: center;
  }

  .day-chart-loading {
    min-height: 20rem;
    border-radius: var(--radius-lg);
  }

  .day-chart-stack {
    display: grid;
    gap: 0.75rem;
  }

  .day-chart-row {
    display: grid;
    gap: 0.35rem;
    min-width: 0;
  }

  .row-heading {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .row-swatch {
    width: 0.58rem;
    height: 0.58rem;
    border-radius: 999px;
  }

  :global(.day-metric-chart) {
    height: 10.5rem;
    min-height: 0;
    aspect-ratio: auto;
  }

  .single-row :global(.day-metric-chart) {
    height: 18rem;
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
    :global(.day-distribution-card [data-slot='card-header']) {
      align-items: stretch;
      flex-direction: column;
    }

    .metric-toggles {
      justify-content: flex-start;
    }

    :global(.day-metric-chart) {
      height: 9.5rem;
    }
  }
</style>
