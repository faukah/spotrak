<script lang="ts">
  import { onMount, tick } from 'svelte';
  import * as echarts from 'echarts/core';
  import { LineChart } from 'echarts/charts';
  import { GridComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { Activity, Clock3, Disc3, Library, Mic2 } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { DiversityTimelinePoint, SummaryStats, TimelinePoint } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import { chartColors, chartTooltip } from '../../lib/charts/theme';
  import { formatChartValue } from '../../lib/stats/format';
  import * as Card from '../ui/card';

  echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer]);

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
  let chartElement: HTMLDivElement | undefined;
  let chart: echarts.ECharts | null = null;
  let resizeObserver: ResizeObserver | null = null;
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

  onMount(() => {
    const handleThemeChange = () => renderChart();
    void load();
    resizeObserver = new ResizeObserver(() => chart?.resize());
    window.addEventListener('spotrak:theme-change', handleThemeChange);
    return () => {
      window.removeEventListener('spotrak:theme-change', handleThemeChange);
      resizeObserver?.disconnect();
      chart?.dispose();
    };
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
      await tick();
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

    await tick();
    if (id !== requestId || chartError || points.length === 0 || !chartElement) return;
    if (!chart) {
      chart = echarts.init(chartElement);
      resizeObserver?.observe(chartElement);
    }
    renderChart();
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

  function formatValue(value: number): string {
    if (active === 'time') return formatChartValue(value, 'duration');
    return formatChartValue(value, 'count');
  }

  function renderChart() {
    if (!chart) return;
    const colors = chartColors();

    chart.setOption({
      backgroundColor: 'transparent',
      animationDuration: 500,
      tooltip: {
        trigger: 'axis',
        ...chartTooltip(colors, (value: number) => formatValue(value)),
      },
      grid: { left: 46, right: 16, top: 18, bottom: 42 },
      xAxis: {
        type: 'category',
        data: points.map((point) => point.label),
        boundaryGap: false,
        axisLabel: { color: colors.muted, hideOverlap: true, fontSize: 11 },
        axisLine: { lineStyle: { color: colors.border } },
        axisTick: { show: false },
      },
      yAxis: {
        type: 'value',
        splitLine: { lineStyle: { color: colors.border, opacity: 0.38 } },
        axisLabel: { color: colors.muted, formatter: (value: number) => formatValue(value) },
      },
      series: [
        {
          name: activeLabel,
          type: 'line',
          data: points.map((point) => point.value),
          smooth: true,
          symbol: 'circle',
          symbolSize: 4,
          showSymbol: points.length <= 80,
          lineStyle: { color: active === 'time' ? colors.accent : colors.primary, width: 2.4 },
          itemStyle: { color: active === 'time' ? colors.accent : colors.primary },
          areaStyle: { color: active === 'time' ? 'rgba(209,167,95,0.16)' : 'rgba(158,185,142,0.16)' },
        },
      ],
    });
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
          <div
            bind:this={chartElement}
            class:dimmed={chartLoading || chartError || points.length === 0}
            class="chart"
            role="img"
            aria-label={`${activeLabel} over time area chart`}
          ></div>
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
