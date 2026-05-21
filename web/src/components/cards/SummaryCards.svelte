<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, Clock3, Disc3, Library, Mic2 } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { DiversityTimelinePoint, SummaryStats, TimelinePoint } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import { formatCountValue, formatDurationValue, formatLongDate, formatShortDate } from '../../lib/charts/theme';
  import * as Card from '../ui/card';

  type SummaryMode = 'plays' | 'time' | 'tracks' | 'artists' | 'albums';
  type ChartPoint = { label: string; value: number };

  export let apiPrefix = '';

  let stats: SummaryStats | null = null;
  let loading = true;
  let chartLoading = true;
  let error: string | null = null;
  let chartError: string | null = null;
  let active: SummaryMode = 'plays';
  let points: ChartPoint[] = [];
  let requestId = 0;

  $: cards = stats
    ? [
        { key: 'plays' as const, label: 'plays', value: stats.total_listens.toLocaleString(), detail: 'total events', icon: Activity },
        { key: 'time' as const, label: 'time', value: formatDuration(stats.total_duration_ms), detail: 'listening duration', icon: Clock3 },
        { key: 'tracks' as const, label: 'tracks', value: stats.unique_tracks.toLocaleString(), detail: 'unique', icon: Disc3 },
        { key: 'artists' as const, label: 'artists', value: stats.unique_artists.toLocaleString(), detail: 'unique', icon: Mic2 },
        { key: 'albums' as const, label: 'albums', value: stats.unique_albums.toLocaleString(), detail: 'unique', icon: Library },
      ]
    : [];

  const timelineWidth = 720;
  const timelineHeight = 300;
  const timelinePadding = { top: 18, right: 16, bottom: 42, left: 46 };

  $: activeLabel = cards.find((card) => card.key === active)?.label ?? 'plays';
  $: timelineColor = active === 'time' ? 'var(--chart-2)' : 'var(--chart-1)';
  $: chartDimmed = chartLoading || chartError || points.length === 0;
  $: timelinePlotWidth = timelineWidth - timelinePadding.left - timelinePadding.right;
  $: timelinePlotHeight = timelineHeight - timelinePadding.top - timelinePadding.bottom;
  $: timelineMax = Math.max(1, ...points.map((point) => point.value));
  $: xTickStep = Math.max(1, Math.ceil(points.length / 6));
  $: yTicks = [0, Math.ceil(timelineMax / 2), Math.ceil(timelineMax)];
  $: timelineLinePath = linePath(points);
  $: timelineAreaPath = areaPath(points);

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = null;
    try {
      stats = await apiFetch<SummaryStats>(`${apiPrefix}/stats/summary`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load summary';
    } finally {
      loading = false;
    }

    if (stats) {
      await selectMode(active);
    }
  }

  async function selectMode(mode: SummaryMode) {
    active = mode;
    const id = ++requestId;
    chartLoading = true;
    chartError = null;
    try {
      points = await fetchPoints(mode);
    } catch (err) {
      chartError = err instanceof Error ? err.message : 'Unable to load timeline';
      points = [];
    } finally {
      if (id === requestId) chartLoading = false;
    }
  }

  async function fetchPoints(mode: SummaryMode): Promise<ChartPoint[]> {
    if (mode === 'plays' || mode === 'time') {
      const timeline = await apiFetch<TimelinePoint[]>(`${apiPrefix}/stats/listening-over-time?split=day`);
      return timeline.map((point) => ({
        label: point.bucket,
        value: mode === 'plays' ? point.count : point.duration_ms,
      }));
    }

    const diversity = await apiFetch<DiversityTimelinePoint[]>(`${apiPrefix}/stats/diversity-over-time?split=day`);
    return diversity.map((point) => ({
      label: point.bucket,
      value: mode === 'tracks' ? point.unique_tracks : mode === 'artists' ? point.unique_artists : point.unique_albums,
    }));
  }

  function formatValue(value: unknown): string {
    if (active === 'time') return formatDurationValue(value);
    return formatCountValue(value);
  }

  function formatTooltipValue(value: unknown): string {
    const formatted = formatValue(value);
    return active === 'time' ? formatted : `${formatted} ${activeLabel}`;
  }

  function pointX(index: number): number {
    const denominator = Math.max(1, points.length - 1);
    return timelinePadding.left + (index / denominator) * timelinePlotWidth;
  }

  function pointY(value: number): number {
    return timelinePadding.top + timelinePlotHeight - (value / timelineMax) * timelinePlotHeight;
  }

  function tickY(value: number): number {
    return pointY(value);
  }

  function linePath(input: ChartPoint[]): string {
    if (input.length === 0) return '';
    return input.map((point, index) => `${index === 0 ? 'M' : 'L'} ${pointX(index)} ${pointY(point.value)}`).join(' ');
  }

  function areaPath(input: ChartPoint[]): string {
    if (input.length === 0) return '';
    const baseline = timelinePadding.top + timelinePlotHeight;
    const firstX = pointX(0);
    const lastX = pointX(input.length - 1);
    return `M ${firstX} ${baseline} ${input.map((point, index) => `L ${pointX(index)} ${pointY(point.value)}`).join(' ')} L ${lastX} ${baseline} Z`;
  }
</script>

{#if loading}
  <div class="summary-grid" aria-live="polite">
    {#each Array(5) as _}
      <div class="summary-card skeleton"></div>
    {/each}
  </div>
{:else if error}
  <Card.Root class="state-card"><Card.Content>{error}</Card.Content></Card.Root>
{:else if stats}
  <section class="summary-section">
    <div class="summary-grid">
      {#each cards as item}
        <button class:active={active === item.key} class="summary-card" type="button" onclick={() => selectMode(item.key)}>
          <svelte:component this={item.icon} class="icon" aria-hidden="true" />
          <span class="label">{item.label}</span>
          <strong>{item.value}</strong>
          <span class="detail">{item.detail}</span>
        </button>
      {/each}
    </div>

    <Card.Root class="timeline-card">
      <Card.Header>
        <Card.Description>daily timeline</Card.Description>
        <Card.Title>{activeLabel} over time</Card.Title>
      </Card.Header>
      <Card.Content>
        <div class="chart-frame">
          <div class="chart" class:dimmed={chartDimmed} role="img" aria-label={`${activeLabel} over time area chart`} style={`--timeline-color: ${timelineColor};`}>
            <svg viewBox={`0 0 ${timelineWidth} ${timelineHeight}`} preserveAspectRatio="xMidYMid meet" aria-hidden="true">
              {#each yTicks as tick}
                <g>
                  <line class="chart-grid-line" x1={timelinePadding.left} x2={timelineWidth - timelinePadding.right} y1={tickY(tick)} y2={tickY(tick)} />
                  <text class="chart-axis-label" x={timelinePadding.left - 8} y={tickY(tick) + 4} text-anchor="end">{formatValue(tick)}</text>
                </g>
              {/each}
              {#if timelineAreaPath}
                <path class="chart-area" d={timelineAreaPath}></path>
                <path class="chart-line" d={timelineLinePath}></path>
              {/if}
              {#each points as point, index}
                {#if index % xTickStep === 0}
                  <text class="chart-axis-label" x={pointX(index)} y={timelineHeight - 12} text-anchor="middle">{formatShortDate(point.label)}</text>
                {/if}
                <circle class="chart-point" cx={pointX(index)} cy={pointY(point.value)} r="3">
                  <title>{formatLongDate(point.label)}: {formatTooltipValue(point.value)}</title>
                </circle>
              {/each}
            </svg>
          </div>
          <table class="sr-only">
            <caption>{activeLabel} timeline data</caption>
            <thead><tr><th scope="col">Date</th><th scope="col">Value</th></tr></thead>
            <tbody>
              {#each points as point}
                <tr><td>{formatLongDate(point.label)}</td><td>{formatTooltipValue(point.value)}</td></tr>
              {/each}
            </tbody>
          </table>
          {#if chartLoading}
            <div class="chart-overlay"><span>Loading timeline…</span></div>
          {:else if chartError}
            <div class="chart-overlay"><p class="state error">{chartError}</p></div>
          {:else if points.length === 0}
            <div class="chart-overlay"><p class="state">No timeline data yet.</p></div>
          {/if}
        </div>
      </Card.Content>
    </Card.Root>
  </section>
{/if}

<style>
  .summary-section {
    display: grid;
    gap: 0.75rem;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .summary-card {
    display: grid;
    gap: 0.35rem;
    min-height: 6.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 0.95rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 94%, transparent);
    color: var(--color-text);
    text-align: left;
    box-shadow: var(--shadow-card);
    cursor: pointer;
    transition: border-color 140ms ease, background 140ms ease, transform 140ms ease;
  }

  .summary-card:hover,
  .summary-card.active {
    border-color: color-mix(in srgb, var(--color-primary) 70%, var(--color-border));
    background: color-mix(in srgb, var(--color-panel-2) 76%, transparent);
  }

  .summary-card:active {
    transform: translateY(1px);
  }

  :global(.icon) {
    width: 1rem;
    height: 1rem;
    color: var(--color-primary);
  }

  .label,
  .detail,
  .state {
    color: var(--color-muted);
  }

  .label,
  .detail {
    font-size: 0.78rem;
    line-height: 1.2;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  strong {
    color: var(--color-text);
    font-size: clamp(1.35rem, 2.6vw, 2.1rem);
    line-height: 0.95;
    letter-spacing: -0.08em;
  }

  .chart-frame {
    position: relative;
    min-height: clamp(18rem, 32vw, 27rem);
  }

  .chart {
    width: 100%;
    min-height: clamp(18rem, 32vw, 27rem);
    opacity: 1;
    transition: opacity 140ms ease;
  }

  .chart.dimmed {
    opacity: 0.22;
  }

  .chart svg {
    width: 100%;
    min-height: clamp(18rem, 32vw, 27rem);
    overflow: visible;
  }

  .chart-grid-line {
    stroke: color-mix(in srgb, var(--color-border) 70%, transparent);
    stroke-width: 1;
  }

  .chart-axis-label {
    fill: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 760;
  }

  .chart-area {
    fill: color-mix(in srgb, var(--timeline-color) 18%, transparent);
  }

  .chart-line {
    fill: none;
    stroke: var(--timeline-color);
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.4;
  }

  .chart-point {
    fill: var(--timeline-color);
    stroke: var(--color-bg-elevated);
    stroke-width: 1.5;
  }

  .chart-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-bg-elevated) 45%, transparent);
    pointer-events: none;
  }

  .chart-overlay span {
    color: var(--color-muted);
    font-size: 0.86rem;
    font-weight: 700;
  }

  .error,
  :global(.state-card) {
    color: var(--color-danger);
  }

  @media (max-width: 980px) {
    .summary-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .summary-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .summary-card {
      padding: 0.75rem;
    }

    .label,
    .detail {
      font-size: 0.68rem;
    }

    strong {
      font-size: clamp(1.15rem, 6vw, 1.65rem);
    }
  }

  @media (max-width: 340px) {
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
