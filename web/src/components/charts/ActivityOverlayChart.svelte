<script lang="ts">
  import { onMount, tick } from 'svelte';
  import * as echarts from 'echarts/core';
  import { BarChart, LineChart } from 'echarts/charts';
  import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { apiFetch } from '../../lib/api/client';
  import type { TimelinePoint } from '../../lib/api/types';
  import { chartColors, chartTooltip } from '../../lib/charts/theme';
  import { formatChartValue } from '../../lib/stats/format';
  import * as Card from '../ui/card';

  echarts.use([BarChart, LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';

  let element: HTMLDivElement | null = null;
  let chart: echarts.ECharts | null = null;
  let points: TimelinePoint[] = [];
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
      points = await apiFetch<TimelinePoint[]>(`/stats/listening-over-time?split=${split}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load activity';
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
    const counts = points.map((point) => point.count);
    const durations = points.map((point) => point.duration_ms);

    chart.setOption({
      backgroundColor: 'transparent',
      animationDuration: 550,
      tooltip: {
        trigger: 'axis',
        ...chartTooltip(colors),
      },
      legend: { top: 0, right: 0, textStyle: { color: colors.muted }, itemWidth: 12, itemHeight: 8 },
      grid: { left: 46, right: 48, top: 38, bottom: 42 },
      xAxis: {
        type: 'category',
        data: labels,
        axisLabel: { color: colors.muted, hideOverlap: true, fontSize: 11 },
        axisLine: { lineStyle: { color: colors.border } },
        axisTick: { show: false },
      },
      yAxis: [
        {
          type: 'value',
          name: 'plays',
          nameTextStyle: { color: colors.muted },
          splitLine: { lineStyle: { color: colors.border, opacity: 0.38 } },
          axisLabel: { color: colors.muted, formatter: (value: number) => formatChartValue(value, 'count') },
        },
        {
          type: 'value',
          name: 'time',
          nameTextStyle: { color: colors.muted },
          splitLine: { show: false },
          axisLabel: { color: colors.muted, formatter: (value: number) => formatChartValue(value, 'duration') },
        },
      ],
      series: [
        {
          name: 'plays',
          type: 'bar',
          yAxisIndex: 0,
          data: counts,
          barMaxWidth: 18,
          itemStyle: { color: 'rgba(158,185,142,0.42)', borderColor: colors.primary, borderWidth: 1, borderRadius: 1 },
        },
        {
          name: 'time',
          type: 'line',
          yAxisIndex: 1,
          data: durations,
          smooth: true,
          symbol: 'circle',
          symbolSize: 4,
          showSymbol: points.length <= 80,
          lineStyle: { color: colors.accent, width: 2.2 },
          itemStyle: { color: colors.accent },
          areaStyle: { color: 'rgba(209,167,95,0.10)' },
        },
      ],
    });
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
      <div bind:this={element} class="chart" role="img" aria-label="Overlaid plays and listening time chart"></div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  .chart {
    width: 100%;
    min-height: clamp(22rem, 43vw, 33rem);
  }

  .state { color: var(--color-muted); }
  .error { color: var(--color-danger); }
</style>
