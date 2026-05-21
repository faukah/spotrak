<script lang="ts">
  import { area as d3Area, curveMonotoneX, line as d3Line } from 'd3-shape';
  import { tickStep } from '../../lib/charts/theme';
  import * as Card from '../ui/card';

  type StatsLinePoint = {
    label: string;
    value: number;
  };

  type ChartPoint = {
    x: number;
    y: number;
  };

  export let title = '';
  export let description = '';
  export let points: StatsLinePoint[] = [];
  export let color = 'var(--chart-1)';
  export let valueLabel = 'Value';
  export let emptyLabel = 'No data yet.';
  export let tableCaption = '';
  export let className = '';
  export let formatValue: (value: unknown) => string = (value) => String(value ?? '');
  export let formatLabel: (value: string) => string = (value) => value;
  export let zeroBased = true;

  const width = 860;
  const height = 285;
  const padding = { top: 18, right: 22, bottom: 42, left: 54 };

  $: plotWidth = width - padding.left - padding.right;
  $: plotHeight = height - padding.top - padding.bottom;
  $: values = points.map((point) => safeValue(point.value));
  $: minDataValue = values.length > 0 ? Math.min(...values) : 0;
  $: maxDataValue = values.length > 0 ? Math.max(...values) : 1;
  $: domainPadding = Math.max(1, (maxDataValue - minDataValue) * 0.12);
  $: minValue = zeroBased ? 0 : Math.max(0, minDataValue - domainPadding);
  $: maxValue = zeroBased ? Math.max(1, maxDataValue) : maxDataValue + domainPadding;
  $: valueRange = Math.max(1, maxValue - minValue);
  $: yTicks = buildTicks(minValue, maxValue, 4);
  $: xTickStep = tickStep(points.length, 8);
  $: pointRadius = points.length > 160 ? 1.05 : points.length > 72 ? 1.45 : 2.65;
  $: line = linePath(points);
  $: area = areaPath(points);
  $: caption = tableCaption || `${title} data`;

  function safeValue(value: number): number {
    return Number.isFinite(value) ? value : 0;
  }

  function pointX(index: number): number {
    const denominator = Math.max(1, points.length - 1);
    return padding.left + (index / denominator) * plotWidth;
  }

  function pointY(value: number): number {
    return padding.top + plotHeight - ((safeValue(value) - minValue) / valueRange) * plotHeight;
  }

  function buildTicks(min: number, max: number, intervals: number): number[] {
    return Array.from({ length: intervals + 1 }, (_, index) => min + ((max - min) / intervals) * index);
  }

  function linePath(input: StatsLinePoint[]): string {
    const path = d3Line<ChartPoint>()
      .x((point) => point.x)
      .y((point) => point.y)
      .curve(curveMonotoneX)(chartPoints(input));
    return path ?? '';
  }

  function areaPath(input: StatsLinePoint[]): string {
    if (input.length === 0) return '';
    const baseline = pointY(minValue);
    const path = d3Area<ChartPoint>()
      .x((point) => point.x)
      .y0(baseline)
      .y1((point) => point.y)
      .curve(curveMonotoneX)(chartPoints(input));
    return path ?? '';
  }

  function chartPoints(input: StatsLinePoint[]): ChartPoint[] {
    return input.map((point, index) => ({
      x: pointX(index),
      y: pointY(point.value),
    }));
  }
</script>

<Card.Root class={`stats-line-card ${className}`}>
  <Card.Header>
    {#if description}<Card.Description>{description}</Card.Description>{/if}
    <Card.Title>{title}</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if points.length === 0}
      <p class="state">{emptyLabel}</p>
    {:else}
      <div class="line-chart" role="img" aria-label={`${title} chart`} style={`--line-color: ${color};`}>
        <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet" aria-hidden="true" focusable="false">
          {#each yTicks as tick (tick)}
            <g>
              <line class="grid-line" x1={padding.left} x2={width - padding.right} y1={pointY(tick)} y2={pointY(tick)} />
              <text class="axis-label" x={padding.left - 8} y={pointY(tick) + 4} text-anchor="end">{formatValue(tick)}</text>
            </g>
          {/each}
          {#if area}
            <path class="area" d={area}></path>
            <path class="line" d={line}></path>
          {/if}
          {#each points as point, index (point.label)}
            {#if index % xTickStep === 0 || index === points.length - 1}
              <text class="axis-label" x={pointX(index)} y={height - 12} text-anchor="middle">{formatLabel(point.label)}</text>
            {/if}
            <circle class="point" cx={pointX(index)} cy={pointY(point.value)} r={pointRadius}>
              <title>{formatLabel(point.label)}: {formatValue(point.value)}</title>
            </circle>
          {/each}
        </svg>
      </div>
      <table class="sr-only">
        <caption>{caption}</caption>
        <thead>
          <tr><th scope="col">Bucket</th><th scope="col">{valueLabel}</th></tr>
        </thead>
        <tbody>
          {#each points as point (point.label)}
            <tr><td>{formatLabel(point.label)}</td><td>{formatValue(point.value)}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.stats-line-card) {
    min-width: 0;
  }

  .line-chart {
    width: 100%;
    color: var(--color-muted);
  }

  svg {
    display: block;
    width: 100%;
    min-height: 12.75rem;
  }

  .grid-line {
    stroke: color-mix(in srgb, var(--color-border) 72%, transparent);
    stroke-width: 1;
  }

  .axis-label {
    fill: var(--color-muted);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
  }

  .area {
    fill: color-mix(in srgb, var(--line-color) 20%, transparent);
  }

  .line {
    fill: none;
    stroke: var(--line-color);
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.1;
  }

  .point {
    fill: var(--color-bg-elevated);
    stroke: var(--line-color);
    stroke-width: 2;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 680px) {
    svg {
      min-height: 10.5rem;
    }
  }
</style>
