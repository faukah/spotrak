<script lang="ts">
  import type { HourRepartitionPoint } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import * as Card from '../ui/card';

  export let points: HourRepartitionPoint[] = [];
  export let title = 'Listening distribution over day';
  export let description = 'local hour';
  export let hourFormat: '12' | '24' = '24';
  export let className = '';

  const width = 720;
  const height = 250;
  const padding = { top: 18, right: 16, bottom: 38, left: 44 };
  const playsColor = 'var(--chart-1)';
  const minutesColor = 'var(--chart-2)';

  $: data = Array.from({ length: 24 }, (_, hour) => {
    const point = points.find((item) => item.hour === hour);
    return {
      hour,
      label: formatHour(hour),
      count: point?.count ?? 0,
      minutes: (point?.duration_ms ?? 0) / 60_000,
      durationMs: point?.duration_ms ?? 0,
    };
  });
  $: plotWidth = width - padding.left - padding.right;
  $: plotHeight = height - padding.top - padding.bottom;
  $: groupWidth = plotWidth / data.length;
  $: maxValue = Math.max(1, ...data.flatMap((point) => [point.count, point.minutes]));
  $: yTicks = [0, maxValue / 2, maxValue];

  function formatHour(hour: number): string {
    if (hourFormat === '24') return `${String(hour).padStart(2, '0')}:00`;
    const suffix = hour < 12 ? 'AM' : 'PM';
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function y(value: number): number {
    return padding.top + plotHeight - (Math.max(0, value) / maxValue) * plotHeight;
  }

  function barHeight(value: number): number {
    return padding.top + plotHeight - y(value);
  }

  function barX(index: number, offset: number): number {
    return padding.left + index * groupWidth + groupWidth * offset;
  }

  function formatAxis(value: number): string {
    return Intl.NumberFormat(undefined, { maximumFractionDigits: value < 10 ? 1 : 0 }).format(value);
  }
</script>

<Card.Root class={`hour-distribution-card ${className}`}>
  <Card.Header>
    <Card.Description>{description}</Card.Description>
    <Card.Title>{title}</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if points.length === 0}
      <p class="state">No hourly listening data yet.</p>
    {:else}
      <div class="legend" aria-hidden="true">
        <span style={`--swatch: ${playsColor};`}>plays</span>
        <span style={`--swatch: ${minutesColor};`}>minutes</span>
      </div>
      <div class="hour-chart" role="img" aria-label="Listening distribution by local hour">
        <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet" aria-hidden="true" focusable="false">
          {#each yTicks as tick (tick)}
            <g>
              <line class="grid-line" x1={padding.left} x2={width - padding.right} y1={y(tick)} y2={y(tick)} />
              <text class="axis-label" x={padding.left - 8} y={y(tick) + 4} text-anchor="end">{formatAxis(tick)}</text>
            </g>
          {/each}
          {#each data as point, index (point.hour)}
            <rect class="play-bar" x={barX(index, 0.18)} y={y(point.count)} width={Math.max(2, groupWidth * 0.24)} height={barHeight(point.count)} rx="2">
              <title>{point.label}: {point.count.toLocaleString()} plays</title>
            </rect>
            <rect class="time-bar" x={barX(index, 0.52)} y={y(point.minutes)} width={Math.max(2, groupWidth * 0.24)} height={barHeight(point.minutes)} rx="2">
              <title>{point.label}: {Math.round(point.minutes).toLocaleString()} minutes</title>
            </rect>
            {#if index % 3 === 0}
              <text class="axis-label" x={padding.left + index * groupWidth + groupWidth / 2} y={height - 12} text-anchor="middle">{point.label}</text>
            {/if}
          {/each}
        </svg>
      </div>
      <table class="sr-only">
        <caption>Hourly listening distribution data</caption>
        <thead>
          <tr><th scope="col">Hour</th><th scope="col">Plays</th><th scope="col">Time</th></tr>
        </thead>
        <tbody>
          {#each data as point (point.hour)}
            <tr><td>{point.label}</td><td>{point.count}</td><td>{formatDuration(point.durationMs)}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  .legend {
    display: flex;
    gap: 0.75rem;
    margin: -0.3rem 0 0.35rem;
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 750;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .legend span {
    display: inline-flex;
    gap: 0.32rem;
    align-items: center;
  }

  .legend span::before {
    width: 0.52rem;
    height: 0.52rem;
    border-radius: 0.12rem;
    background: var(--swatch);
    content: '';
  }

  .hour-chart {
    width: 100%;
  }

  svg {
    display: block;
    width: 100%;
    min-height: 13rem;
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

  .play-bar {
    fill: var(--chart-1);
  }

  .time-bar {
    fill: var(--chart-2);
    opacity: 0.82;
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
