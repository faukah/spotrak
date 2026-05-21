<script lang="ts">
  import { onMount } from 'svelte';
  import { AreaChart } from 'layerchart';
  import { Activity, Clock3, Disc3, Library, Mic2 } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { DiversityTimelinePoint, SummaryStats, TimelinePoint } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import { formatCountValue, formatDurationValue, formatLongDate, formatShortDate, tickStep } from '../../lib/charts/theme';
  import { cn } from '../../lib/utils';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type SummaryMode = 'plays' | 'time' | 'tracks' | 'artists' | 'albums';
  type ChartPoint = { label: string; value: number };
  type TooltipItem = { color?: string };

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

  $: activeLabel = cards.find((card) => card.key === active)?.label ?? 'plays';
  $: timelineConfig = {
    value: {
      label: activeLabel,
      color: active === 'time' ? 'var(--chart-2)' : 'var(--chart-1)',
    },
  } satisfies Chart.ChartConfig;
  $: timelineSeries = [
    {
      key: 'value',
      label: activeLabel,
      value: 'value',
      color: timelineConfig.value.color,
    },
  ];
  $: chartClass = cn('chart', (chartLoading || chartError || points.length === 0) && 'dimmed');
  $: xTickStep = tickStep(points.length, 6);

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

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
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
          <Chart.Container
            config={timelineConfig}
            class={chartClass}
            role="img"
            aria-label={`${activeLabel} over time area chart`}
          >
            <AreaChart
              data={points}
              x="label"
              y="value"
              yBaseline={0}
              series={timelineSeries}
              padding={{ left: 46, right: 16, top: 18, bottom: 42 }}
              props={{
                xAxis: { format: formatShortDate, ticks: xTickStep },
                yAxis: { format: formatValue, tickSpacing: 72 },
                area: { fillOpacity: 0.18 },
                line: { strokeWidth: 2.4 },
              }}
            >
              {#snippet tooltip()}
                <Chart.Tooltip labelFormatter={formatLongDate}>
                  {#snippet formatter({ value, name, item })}
                    <div class="tooltip-row">
                      <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                      <span class="tooltip-name">{name}</span>
                      <span class="tooltip-value">{formatTooltipValue(value)}</span>
                    </div>
                  {/snippet}
                </Chart.Tooltip>
              {/snippet}
            </AreaChart>
          </Chart.Container>
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

  :global(.chart) {
    width: 100%;
    min-height: clamp(18rem, 32vw, 27rem);
    opacity: 1;
    transition: opacity 140ms ease;
  }

  :global(.chart.dimmed) {
    opacity: 0.22;
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
