<script lang="ts">
  import { onMount } from 'svelte';
  import { LineChart } from 'layerchart';
  import { apiFetch } from '../../lib/api/client';
  import type { DiversityTimelinePoint } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatLongDate, formatShortDate, numericValue, tickStep } from '../../lib/charts/theme';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type TooltipItem = { color?: string };

  export let split: 'year' | 'month' | 'week' | 'day' | 'hour' = 'day';

  let points: DiversityTimelinePoint[] = [];
  let loading = true;
  let error: string | null = null;

  const uniqueConfig = {
    unique_tracks: { label: 'tracks', color: chartColor(0) },
    unique_artists: { label: 'artists', color: chartColor(2) },
    unique_albums: { label: 'albums', color: chartColor(4) },
  } satisfies Chart.ChartConfig;

  const releaseYearConfig = {
    average_release_year: { label: 'avg release year', color: chartColor(1) },
  } satisfies Chart.ChartConfig;

  const uniqueSeries = [
    { key: 'unique_tracks', label: uniqueConfig.unique_tracks.label, value: 'unique_tracks', color: uniqueConfig.unique_tracks.color },
    { key: 'unique_artists', label: uniqueConfig.unique_artists.label, value: 'unique_artists', color: uniqueConfig.unique_artists.color },
    { key: 'unique_albums', label: uniqueConfig.unique_albums.label, value: 'unique_albums', color: uniqueConfig.unique_albums.color },
  ];

  const releaseYearSeries = [
    { key: 'average_release_year', label: releaseYearConfig.average_release_year.label, value: 'average_release_year', color: releaseYearConfig.average_release_year.color },
  ];

  $: releaseYearData = points
    .filter((point) => point.average_release_year !== null && point.average_release_year !== undefined)
    .map((point) => ({
      bucket: point.bucket,
      average_release_year: point.average_release_year ?? 0,
    }));
  $: uniqueTickStep = tickStep(points.length, 6);
  $: releaseYearTickStep = tickStep(releaseYearData.length, 6);

  onMount(() => {
    void load();
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
  }

  function formatYearValue(value: unknown): string {
    return numericValue(value).toLocaleString(undefined, { maximumFractionDigits: 1 });
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
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
      <div class="chart-stack">
        <Chart.Container config={uniqueConfig} class="chart" role="img" aria-label="Unique tracks artists and albums chart">
          <LineChart
            data={points}
            x="bucket"
            series={uniqueSeries}
            yBaseline={0}
            legend
            padding={{ left: 42, right: 24, top: 38, bottom: 42 }}
            props={{
              xAxis: { format: formatShortDate, ticks: uniqueTickStep },
              yAxis: { format: formatCountValue, tickSpacing: 72 },
              spline: { strokeWidth: 2 },
            }}
          >
            {#snippet tooltip()}
              <Chart.Tooltip labelFormatter={formatLongDate}>
                {#snippet formatter({ value, name, item })}
                  <div class="tooltip-row">
                    <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                    <span class="tooltip-name">{name}</span>
                    <span class="tooltip-value">{formatCountValue(value)}</span>
                  </div>
                {/snippet}
              </Chart.Tooltip>
            {/snippet}
          </LineChart>
        </Chart.Container>

        {#if releaseYearData.length > 0}
          <Chart.Container config={releaseYearConfig} class="release-chart" role="img" aria-label="Average release year chart">
            <LineChart
              data={releaseYearData}
              x="bucket"
              series={releaseYearSeries}
              legend
              padding={{ left: 42, right: 24, top: 12, bottom: 32 }}
              props={{
                xAxis: { format: formatShortDate, ticks: releaseYearTickStep },
                yAxis: { format: formatYearValue, tickSpacing: 56 },
                spline: { strokeWidth: 2, 'stroke-dasharray': '5 5' },
              }}
            >
              {#snippet tooltip()}
                <Chart.Tooltip labelFormatter={formatLongDate}>
                  {#snippet formatter({ value, name, item })}
                    <div class="tooltip-row">
                      <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                      <span class="tooltip-name">{name}</span>
                      <span class="tooltip-value">{formatYearValue(value)}</span>
                    </div>
                  {/snippet}
                </Chart.Tooltip>
              {/snippet}
            </LineChart>
          </Chart.Container>
        {/if}
        <table class="sr-only">
          <caption>Diversity data by bucket</caption>
          <thead>
            <tr>
              <th scope="col">Date</th>
              <th scope="col">Unique tracks</th>
              <th scope="col">Unique artists</th>
              <th scope="col">Unique albums</th>
              <th scope="col">Average release year</th>
            </tr>
          </thead>
          <tbody>
            {#each points as point}
              <tr>
                <td>{formatLongDate(point.bucket)}</td>
                <td>{point.unique_tracks}</td>
                <td>{point.unique_artists}</td>
                <td>{point.unique_albums}</td>
                <td>{point.average_release_year ?? 'Not available'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  .chart-stack {
    display: grid;
    gap: 0.75rem;
  }

  :global(.chart) {
    width: 100%;
    min-height: 22rem;
  }

  :global(.release-chart) {
    width: 100%;
    min-height: 12rem;
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

  .state { color: var(--color-muted); }
  .error { color: var(--color-danger); }
</style>
