<script lang="ts">
  import { BarChart, LineChart } from 'layerchart';
  import { curveMonotoneX } from 'd3-shape';
  import { bandTickStride } from '../../lib/charts/ticks';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type StatsMetricPoint = {
    label: string;
    value: number;
    rawLabel?: string;
  };

  type ChartKind = 'bar' | 'line';
  type TooltipItem = { color?: string };

  export let title = '';
  export let description = '';
  export let points: StatsMetricPoint[] = [];
  export let valueLabel = 'Value';
  export let color = 'var(--chart-1)';
  export let kind: ChartKind = 'bar';
  export let emptyLabel = 'No data for this range.';
  export let className = '';
  export let formatValue: (value: unknown) => string = (value) => String(value ?? '');
  export let formatAxisValue: ((value: unknown) => string) | undefined = undefined;
  export let formatTooltipValue: ((value: unknown) => string) | undefined = undefined;
  export let zeroBased = true;
  export let tableCaption = '';

  let chartWidth = 720;

  $: chartData = points.map((point) => ({
    label: point.label,
    value: safeValue(point.value),
    rawLabel: point.rawLabel ?? point.label,
  }));
  $: values = chartData.map((point) => point.value);
  $: minDataValue = values.length > 0 ? Math.min(...values) : 0;
  $: maxDataValue = values.length > 0 ? Math.max(...values) : 1;
  $: domainRange = Math.max(0, maxDataValue - minDataValue);
  $: domainPadding = Math.max(maxDataValue > 10 ? 1 : 0.1, domainRange * 0.12);
  $: yDomain = zeroBased ? [0, null] : [Math.max(0, minDataValue - domainPadding), maxDataValue + domainPadding];
  $: xTickStride = bandTickStride(chartData.length, chartWidth, {
    minTickSpacing: 52,
    maxTicks: 12,
    minTicks: 4,
    horizontalPadding: 78,
  });
  $: caption = tableCaption || `${title} data`;
  $: axisFormatter = formatAxisValue ?? formatValue;
  $: tooltipFormatter = formatTooltipValue ?? formatValue;
  $: chartConfig = {
    value: { label: valueLabel, color },
  } satisfies Chart.ChartConfig;
  $: series = [{ key: 'value', label: valueLabel, value: 'value', color }];

  function safeValue(value: number): number {
    return Number.isFinite(value) ? value : 0;
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? color;
  }
</script>

<Card.Root class={`stats-metric-card ${className} overflow-visible`}>
  <Card.Header>
    {#if description}<Card.Description>{description}</Card.Description>{/if}
    <Card.Title>{title}</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if chartData.length === 0}
      <p class="state">{emptyLabel}</p>
    {:else}
      <div class="chart-frame" bind:clientWidth={chartWidth}>
        <Chart.Container
          config={chartConfig}
          class="metric-chart"
          role="group"
          aria-label={`${title} chart`}
        >
          {#if kind === 'bar'}
            <BarChart
              data={chartData}
              x="label"
              y="value"
              {series}
              yDomain={yDomain as [number, number | null]}
              bandPadding={0.34}
              padding={{ left: 54, right: 18, top: 14, bottom: 40 }}
              grid={{ y: true }}
              props={{
                xAxis: { ticks: xTickStride },
                yAxis: { format: axisFormatter, ticks: 4 },
                bars: { radius: 5, strokeWidth: 0 },
                tooltip: { context: { mode: 'band' } },
              }}
            >
              {#snippet tooltip()}
                <Chart.Tooltip contained="window" labelFormatter={(value) => String(value)} indicator="line">
                  {#snippet formatter({ value, name, item })}
                    <div class="tooltip-row">
                      <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                      <span class="tooltip-name">{name}</span>
                      <span class="tooltip-value">{tooltipFormatter(value)}</span>
                    </div>
                  {/snippet}
                </Chart.Tooltip>
              {/snippet}
            </BarChart>
          {:else}
            <LineChart
              data={chartData}
              x="label"
              y="value"
              {series}
              yDomain={yDomain as [number, number | null]}
              padding={{ left: 54, right: 22, top: 16, bottom: 40 }}
              grid={{ y: true }}
              props={{
                xAxis: { ticks: xTickStride },
                yAxis: { format: axisFormatter, ticks: 4 },
                spline: { curve: curveMonotoneX, strokeWidth: 2.2 },
                points: { r: 3 },
                tooltip: { context: { mode: 'quadtree-x' } },
              }}
            >
              {#snippet tooltip()}
                <Chart.Tooltip contained="window" labelFormatter={(value) => String(value)} indicator="line">
                  {#snippet formatter({ value, name, item })}
                    <div class="tooltip-row">
                      <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                      <span class="tooltip-name">{name}</span>
                      <span class="tooltip-value">{tooltipFormatter(value)}</span>
                    </div>
                  {/snippet}
                </Chart.Tooltip>
              {/snippet}
            </LineChart>
          {/if}
        </Chart.Container>
      </div>
      <table class="sr-only">
        <caption>{caption}</caption>
        <thead>
          <tr><th scope="col">Bucket</th><th scope="col">{valueLabel}</th></tr>
        </thead>
        <tbody>
          {#each chartData as point (point.rawLabel)}
            <tr><td>{point.label}</td><td>{tooltipFormatter(point.value)}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.stats-metric-card) {
    min-width: 0;
    height: 100%;
  }

  .chart-frame {
    min-width: 0;
    padding: 0.1rem 0 0;
  }

  :global(.metric-chart) {
    width: 100%;
    min-width: 0;
    height: 17rem;
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
    :global(.metric-chart) {
      height: 14rem;
    }
  }
</style>
