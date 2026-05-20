<script lang="ts">
  import { onMount } from 'svelte';
  import { BarChart, PieChart } from 'layerchart';
  import { apiFetch } from '../../lib/api/client';
  import type { AlbumReleaseYearsStats, FeatureRatioStats, HourRepartitionPoint, LongestSession, MeResponse } from '../../lib/api/types';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { selectedStatsMetric } from '../../lib/stores/preferences';
  import { chartColor, formatMetricValue, tickStep } from '../../lib/charts/theme';
  import { transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type Metric = 'count' | 'duration';

  let featureRatio: FeatureRatioStats | null = null;
  let releaseYears: AlbumReleaseYearsStats | null = null;
  let sessions: LongestSession[] = [];
  let hours: HourRepartitionPoint[] = [];
  let timezone: string | null = null;
  let loading = true;
  let error: string | null = null;
  let metric: Metric = 'count';
  let unsubscribe: (() => void) | undefined;

  const featureConfig = {
    solo: { label: 'Solo', color: chartColor(0) },
    features: { label: 'Features', color: chartColor(1) },
  } satisfies Chart.ChartConfig;

  const barConfig = {
    value: { label: 'Value', color: chartColor(1) },
  } satisfies Chart.ChartConfig;

  const hourConfig = {
    value: { label: 'Value', color: chartColor(0) },
  } satisfies Chart.ChartConfig;

  $: featureData = featureRatio
    ? [
        { key: 'solo' as const, label: 'Solo', value: metricValue(featureRatio.solo_count, featureRatio.solo_duration_ms) },
        { key: 'features' as const, label: 'Features', value: metricValue(featureRatio.feature_count, featureRatio.feature_duration_ms) },
      ]
    : [];

  $: releaseYearData = releaseYears
    ? releaseYears.distribution
      .filter((point) => point.release_year !== null && point.release_year !== undefined)
      .map((point) => ({
        label: String(point.release_year),
        value: metricValue(point.count, point.duration_ms),
      }))
    : [];

  $: hourData = Array.from({ length: 24 }, (_, hour) => {
    const point = hours.find((item) => item.hour === hour);
    return {
      label: String(hour),
      value: point ? metricValue(point.count, point.duration_ms) : 0,
    };
  });
  $: releaseYearTickStep = tickStep(releaseYearData.length, 6);
  $: hourTickStep = tickStep(hourData.length, 6);

  function sessionTrackTransition(sessionIndex: number, trackIndex: number, trackId: string): string {
    return viewTransitionName(trackId, `session-${sessionIndex}-${trackIndex}`);
  }

  onMount(() => {
    unsubscribe = selectedStatsMetric.subscribe((value) => {
      metric = value;
    });
    void load();
    return () => {
      unsubscribe?.();
    };
  });

  async function load() {
    loading = true;
    error = null;
    try {
      const [me, nextFeatureRatio, nextReleaseYears, nextSessions, nextHours] = await Promise.all([
        apiFetch<MeResponse>('/users/me'),
        apiFetch<FeatureRatioStats>('/stats/feature-ratio'),
        apiFetch<AlbumReleaseYearsStats>('/stats/album-release-years'),
        apiFetch<LongestSession[]>('/stats/longest-sessions?limit=4'),
        apiFetch<HourRepartitionPoint[]>('/stats/hour-repartition/tracks'),
      ]);
      timezone = me.settings.timezone ?? null;
      featureRatio = nextFeatureRatio;
      releaseYears = nextReleaseYears;
      sessions = nextSessions;
      hours = nextHours;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load extra stats';
    } finally {
      loading = false;
    }
  }

  function metricValue(count: number, durationMs: number): number {
    return metric === 'duration' ? durationMs : count;
  }

  function formatMetric(value: unknown): string {
    return formatMetricValue(value, metric);
  }

  function formatFeature(value: unknown): string {
    const formatted = formatMetric(value);
    return metric === 'duration' ? formatted : `${formatted} plays`;
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
      <Card.Content>
        <Chart.Container config={featureConfig} class="mini-chart">
          <PieChart
            data={featureData}
            key="key"
            label="label"
            value="value"
            innerRadius={0.58}
            legend
            props={{ tooltip: { item: { format: formatFeature } } }}
          />
        </Chart.Container>
      </Card.Content>
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
        <Chart.Container config={barConfig} class="mini-chart">
          <BarChart
            data={releaseYearData}
            x="label"
            y="value"
            yBaseline={0}
            bandPadding={0.28}
            series={[{ key: 'value', label: metric, value: 'value', color: barConfig.value.color }]}
            padding={{ left: 42, right: 12, top: 12, bottom: 34 }}
            props={{
              xAxis: { ticks: releaseYearTickStep },
              yAxis: { format: formatMetric, tickSpacing: 48 },
              tooltip: { item: { format: formatMetric } },
            }}
          />
        </Chart.Container>
      </Card.Content>
    </Card.Root>

    <Card.Root class="graph-card extras-wide">
      <Card.Header>
        <Card.Description>local hour</Card.Description>
        <Card.Title>Listening clock</Card.Title>
      </Card.Header>
      <Card.Content>
        <Chart.Container config={hourConfig} class="mini-chart">
          <BarChart
            data={hourData}
            x="label"
            y="value"
            yBaseline={0}
            bandPadding={0.2}
            series={[{ key: 'value', label: metric, value: 'value', color: hourConfig.value.color }]}
            padding={{ left: 36, right: 10, top: 12, bottom: 30 }}
            props={{
              xAxis: { ticks: hourTickStep },
              yAxis: { format: formatMetric, tickSpacing: 48 },
              tooltip: { item: { format: formatMetric } },
            }}
          />
        </Chart.Container>
      </Card.Content>
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
          {#each sessions as session, sessionIndex (`${session.start}-${session.end}`)}
            <li>
              <strong>{formatDuration(session.duration_ms)}</strong>
              <span>{session.listens} listens · {formatDateTime(session.start, timezone)}</span>
              <div class="session-tracks">
                {#each session.tracks.slice(0, 4) as track, trackIndex (`${track.id}-${track.played_at}`)}
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

  :global(.mini-chart) {
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
