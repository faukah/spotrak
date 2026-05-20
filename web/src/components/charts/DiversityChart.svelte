<script lang="ts">
  import { onMount, tick } from 'svelte';
  import * as echarts from 'echarts/core';
  import { LineChart } from 'echarts/charts';
  import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { apiFetch } from '../../lib/api/client';
  import type { DiversityTimelinePoint } from '../../lib/api/types';
  import { chartColors, chartTooltip } from '../../lib/charts/theme';
  import * as Card from '../ui/card';

  echarts.use([LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';

  let element: HTMLDivElement | null = null;
  let chart: echarts.ECharts | null = null;
  let points: DiversityTimelinePoint[] = [];
  let loading = true;
  let error: string | null = null;
  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    const handleThemeChange = () => render();
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
      points = await apiFetch<DiversityTimelinePoint[]>(`/stats/diversity-over-time?split=${split}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load diversity';
    } finally {
      loading = false;
    }
    await tick();
    if (error || points.length === 0 || !element) return;
    if (!chart) {
      chart = echarts.init(element);
      resizeObserver?.observe(element);
    }
    render();
  }

  function render() {
    if (!chart) return;
    const colors = chartColors();
    const labels = points.map((point) => point.bucket);

    chart.setOption({
      backgroundColor: 'transparent',
      animationDuration: 550,
      tooltip: {
        trigger: 'axis',
        ...chartTooltip(colors),
      },
      legend: { top: 0, right: 0, textStyle: { color: colors.muted }, itemWidth: 12, itemHeight: 8 },
      grid: { left: 42, right: 48, top: 38, bottom: 42 },
      xAxis: {
        type: 'category',
        data: labels,
        boundaryGap: false,
        axisLabel: { color: colors.muted, hideOverlap: true, fontSize: 11 },
        axisLine: { lineStyle: { color: colors.border } },
        axisTick: { show: false },
      },
      yAxis: [
        {
          type: 'value',
          name: 'unique',
          nameTextStyle: { color: colors.muted },
          splitLine: { lineStyle: { color: colors.border, opacity: 0.38 } },
          axisLabel: { color: colors.muted },
        },
        {
          type: 'value',
          name: 'release year',
          nameTextStyle: { color: colors.muted },
          splitLine: { show: false },
          axisLabel: { color: colors.muted },
          min: (value: { min: number }) => Math.floor(value.min - 1),
          max: (value: { max: number }) => Math.ceil(value.max + 1),
        },
      ],
      series: [
        { name: 'tracks', type: 'line', smooth: true, data: points.map((point) => point.unique_tracks), lineStyle: { width: 2, color: colors.primary }, itemStyle: { color: colors.primary }, symbolSize: 4 },
        { name: 'artists', type: 'line', smooth: true, data: points.map((point) => point.unique_artists), lineStyle: { width: 2, color: '#7fa0b8' }, itemStyle: { color: '#7fa0b8' }, symbolSize: 4 },
        { name: 'albums', type: 'line', smooth: true, data: points.map((point) => point.unique_albums), lineStyle: { width: 2, color: '#a68cc2' }, itemStyle: { color: '#a68cc2' }, symbolSize: 4 },
        { name: 'avg release year', type: 'line', yAxisIndex: 1, smooth: true, data: points.map((point) => point.average_release_year), lineStyle: { width: 2, color: colors.accent, type: 'dashed' }, itemStyle: { color: colors.accent }, symbolSize: 4 },
      ],
    });
  }
</script>

<Card.Root class="diversity-card">
  <Card.Header>
    <Card.Description>{split} buckets</Card.Description>
    <Card.Title>Diversity over time</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="chart skeleton"></div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if points.length === 0}
      <p class="state">No diversity data yet.</p>
    {:else}
      <div bind:this={element} class="chart" role="img" aria-label="Unique tracks artists albums and average release year chart"></div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  .chart {
    width: 100%;
    min-height: 22rem;
  }

  .state { color: var(--color-muted); }
  .error { color: var(--color-danger); }
</style>
