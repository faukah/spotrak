<script lang="ts">
  import { onMount, tick } from 'svelte';
  import * as echarts from 'echarts/core';
  import { BarChart, PieChart } from 'echarts/charts';
  import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { apiFetch } from '../../lib/api/client';
  import type { AlbumReleaseYearsStats, FeatureRatioStats, HourRepartitionPoint, LongestSession } from '../../lib/api/types';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { selectedStatsMetric } from '../../lib/stores/preferences';
  import { chartColors as getChartColors, metricTooltip } from '../../lib/charts/theme';
  import { formatChartValue } from '../../lib/stats/format';
  import { transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';

  echarts.use([BarChart, PieChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

  let featureRatio: FeatureRatioStats | null = null;
  let releaseYears: AlbumReleaseYearsStats | null = null;
  let sessions: LongestSession[] = [];
  let hours: HourRepartitionPoint[] = [];
  let loading = true;
  let error: string | null = null;
  let metric: 'count' | 'duration' = 'count';
  let unsubscribe: (() => void) | undefined;

  let featureElement: HTMLDivElement | undefined;
  let yearElement: HTMLDivElement | undefined;
  let hourElement: HTMLDivElement | undefined;
  let featureChart: echarts.ECharts | null = null;
  let yearChart: echarts.ECharts | null = null;
  let hourChart: echarts.ECharts | null = null;
  let resizeObserver: ResizeObserver | null = null;

  function sessionTrackTransition(sessionIndex: number, trackIndex: number, trackId: string): string {
    return viewTransitionName(trackId, `session-${sessionIndex}-${trackIndex}`);
  }

  onMount(() => {
    const handleThemeChange = () => renderCharts();
    unsubscribe = selectedStatsMetric.subscribe((value) => {
      metric = value;
      renderCharts();
    });
    void load();
    resizeObserver = new ResizeObserver(() => {
      featureChart?.resize();
      yearChart?.resize();
      hourChart?.resize();
    });
    window.addEventListener('spotrak:theme-change', handleThemeChange);
    return () => {
      window.removeEventListener('spotrak:theme-change', handleThemeChange);
      cleanup();
    };
  });

  async function load() {
    loading = true;
    error = null;
    try {
      [featureRatio, releaseYears, sessions, hours] = await Promise.all([
        apiFetch<FeatureRatioStats>('/stats/feature-ratio'),
        apiFetch<AlbumReleaseYearsStats>('/stats/album-release-years'),
        apiFetch<LongestSession[]>('/stats/longest-sessions?limit=4'),
        apiFetch<HourRepartitionPoint[]>('/stats/hour-repartition/tracks'),
      ]);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load extra stats';
    } finally {
      loading = false;
    }

    await tick();
    renderCharts();
  }

  function cleanup() {
    unsubscribe?.();
    resizeObserver?.disconnect();
    featureChart?.dispose();
    yearChart?.dispose();
    hourChart?.dispose();
    featureChart = null;
    yearChart = null;
    hourChart = null;
  }

  function renderCharts() {
    if (loading || error) return;
    const colors = getChartColors();
    const tooltip = metricTooltip(colors, metric);

    if (featureRatio && featureElement) {
      featureChart ??= echarts.init(featureElement);
      resizeObserver?.observe(featureElement);
      featureChart.setOption({
        color: [colors.primary, colors.accent],
        tooltip,
        legend: { bottom: 0, textStyle: { color: colors.muted }, itemWidth: 10, itemHeight: 10 },
        series: [
          {
            type: 'pie',
            radius: ['58%', '82%'],
            center: ['50%', '44%'],
            avoidLabelOverlap: true,
            label: { color: colors.text, formatter: '{d}%' },
            labelLine: { lineStyle: { color: colors.border } },
            data: [
              { name: 'solo', value: metric === 'duration' ? featureRatio.solo_duration_ms : featureRatio.solo_count },
              { name: 'features', value: metric === 'duration' ? featureRatio.feature_duration_ms : featureRatio.feature_count },
            ],
          },
        ],
      });
    }

    if (releaseYears && yearElement) {
      yearChart ??= echarts.init(yearElement);
      resizeObserver?.observe(yearElement);
      const distribution = releaseYears.distribution.filter((point) => point.release_year != null);
      yearChart.setOption({
        color: [colors.accent],
        tooltip,
        grid: { left: 42, right: 12, top: 12, bottom: 34 },
        xAxis: {
          type: 'category',
          data: distribution.map((point) => String(point.release_year)),
          axisLabel: { color: colors.muted, hideOverlap: true, fontSize: 10 },
          axisLine: { lineStyle: { color: colors.border } },
          axisTick: { show: false },
        },
        yAxis: {
          type: 'value',
          splitLine: { lineStyle: { color: colors.border, opacity: 0.42 } },
          axisLabel: { color: colors.muted, fontSize: 10, formatter: (value: number) => formatChartValue(value, metric) },
        },
        series: [
          {
            type: 'bar',
            data: distribution.map((point) => (metric === 'duration' ? point.duration_ms : point.count)),
            barMaxWidth: 18,
            itemStyle: { borderRadius: 1 },
          },
        ],
      });
    }

    if (hourElement) {
      hourChart ??= echarts.init(hourElement);
      resizeObserver?.observe(hourElement);
      const byHour = new Map(hours.map((point) => [point.hour, point]));
      const allHours = Array.from({ length: 24 }, (_, hour) => hour);
      hourChart.setOption({
        color: [colors.primary],
        tooltip,
        grid: { left: 36, right: 10, top: 12, bottom: 30 },
        xAxis: {
          type: 'category',
          data: allHours.map((hour) => `${hour}`),
          axisLabel: { color: colors.muted, fontSize: 10 },
          axisLine: { lineStyle: { color: colors.border } },
          axisTick: { show: false },
        },
        yAxis: {
          type: 'value',
          splitLine: { lineStyle: { color: colors.border, opacity: 0.42 } },
          axisLabel: { color: colors.muted, fontSize: 10, formatter: (value: number) => formatChartValue(value, metric) },
        },
        series: [
          {
            type: 'bar',
            data: allHours.map((hour) => {
              const point = byHour.get(hour);
              return point ? (metric === 'duration' ? point.duration_ms : point.count) : 0;
            }),
            barMaxWidth: 16,
            itemStyle: { borderRadius: 1 },
          },
        ],
      });
    }
  }
</script>

{#if loading}
  <div class="extras-grid">
    {#each Array(4) as _}<div class="extra-skeleton skeleton"></div>{/each}
  </div>
{:else if error}
  <Card.Root><Card.Content><p class="error">{error}</p></Card.Content></Card.Root>
{:else}
  <div class="extras-grid">
    <Card.Root class="graph-card">
      <Card.Header>
        <Card.Description>{metric}</Card.Description>
        <Card.Title>Track makeup</Card.Title>
      </Card.Header>
      <Card.Content><div bind:this={featureElement} class="mini-chart"></div></Card.Content>
    </Card.Root>

    <Card.Root class="graph-card">
      <Card.Header>
        <Card.Description>release year buckets</Card.Description>
        <Card.Title>Album eras</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if releaseYears?.average_release_year}
          <p class="average">avg {releaseYears.average_release_year.toFixed(1)}</p>
        {/if}
        <div bind:this={yearElement} class="mini-chart"></div>
      </Card.Content>
    </Card.Root>

    <Card.Root class="graph-card extras-wide">
      <Card.Header>
        <Card.Description>local hour</Card.Description>
        <Card.Title>Listening clock</Card.Title>
      </Card.Header>
      <Card.Content><div bind:this={hourElement} class="mini-chart"></div></Card.Content>
    </Card.Root>
  </div>

  <Card.Root class="sessions-card">
    <Card.Header>
      <Card.Description>gaps under 10 minutes</Card.Description>
      <Card.Title>Longest sessions</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if sessions.length === 0}
        <p class="state">No sessions yet.</p>
      {:else}
        <ol class="sessions">
          {#each sessions as session, sessionIndex}
            <li>
              <strong>{formatDuration(session.duration_ms)}</strong>
              <span>{session.listens} listens · {formatDateTime(session.start)}</span>
              <div class="session-tracks">
                {#each session.tracks.slice(0, 4) as track, trackIndex}
                  <CoverArt src={track.image_url} name={track.track_name} href={transitionHref(`/track/${track.track_id}`, sessionTrackTransition(sessionIndex, trackIndex, track.track_id))} size="xs" transitionName={sessionTrackTransition(sessionIndex, trackIndex, track.track_id)} />
                {/each}
                <small>{session.tracks.map((track) => track.track_name).slice(0, 3).join(' · ')}</small>
              </div>
            </li>
          {/each}
        </ol>
      {/if}
    </Card.Content>
  </Card.Root>
{/if}

<style>
  .extras-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  :global(.extras-wide) {
    grid-column: span 1;
  }

  .mini-chart {
    width: 100%;
    min-height: 15rem;
  }

  .average {
    margin: -0.4rem 0 0.4rem;
    color: var(--color-muted);
    font-size: 0.82rem;
  }

  :global(.sessions-card) {
    margin-top: 0.75rem;
  }

  .sessions {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.65rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .sessions li {
    display: grid;
    gap: 0.35rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.7rem;
    background: color-mix(in srgb, var(--color-panel) 70%, transparent);
  }

  .sessions strong {
    font-size: 1.45rem;
    line-height: 1;
    letter-spacing: -0.07em;
  }

  .sessions span,
  .sessions small,
  .state {
    color: var(--color-muted);
    font-size: 0.78rem;
  }

  .session-tracks {
    display: flex;
    gap: 0.28rem;
    align-items: center;
    min-width: 0;
  }

  .session-tracks small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .extra-skeleton {
    min-height: 21rem;
    border-radius: var(--radius-lg);
  }

  .error {
    color: var(--color-danger);
  }

  @media (max-width: 1050px) {
    .extras-grid,
    .sessions {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 650px) {
    .extras-grid,
    .sessions {
      grid-template-columns: 1fr;
    }
  }
</style>
