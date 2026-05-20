<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Check, ChevronDown } from '@lucide/svelte';
  import * as echarts from 'echarts/core';
  import { BarChart } from 'echarts/charts';
  import { GridComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import { apiFetch } from '../../lib/api/client';
  import type {
    EntityStats,
    HistoryEvent,
    HourRepartitionPoint,
    MeResponse,
    OverviewStatsResponse,
    StatsRangeKey,
    StatsRangeResponse,
    SummaryStats,
    TopArtist,
    TopTrack,
  } from '../../lib/api/types';
  import { chartColors, chartTooltip } from '../../lib/charts/theme';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { transitionHref, viewTransitionName } from '../../lib/images';
  import { formatChartValue } from '../../lib/stats/format';
  import CoverArt from '../media/CoverArt.svelte';
  import { Button } from '../ui/button';
  import * as Card from '../ui/card';

  echarts.use([BarChart, GridComponent, TooltipComponent, CanvasRenderer]);

  type Trend = { text: string; tone: 'up' | 'down' | 'flat' } | null;

  const currentYear = new Date().getFullYear();
  const rangeButtons: { key: StatsRangeKey; label: string }[] = [
    { key: 'today', label: 'Today' },
    { key: 'week', label: 'This week' },
    { key: 'month', label: 'This month' },
    { key: 'year', label: 'This year' },
    { key: 'all', label: 'All' },
  ];

  let rangeKey: StatsRangeKey = 'today';
  let selectedYear = currentYear;
  let availableYears: number[] = [currentYear];
  let hourFormat: '12' | '24' = '24';

  let summary: SummaryStats | null = null;
  let previousSummary: SummaryStats | null = null;
  let bestArtist: TopArtist | null = null;
  let bestArtistStats: EntityStats | null = null;
  let bestSong: TopTrack | null = null;
  let hours: HourRepartitionPoint[] = [];
  let history: HistoryEvent[] = [];

  let loading = true;
  let error: string | null = null;
  let requestId = 0;

  let hourElement: HTMLDivElement | null = null;
  let rangeMenuElement: HTMLDivElement | null = null;
  let hourChart: echarts.ECharts | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let rangeMenuOpen = false;
  let activeRange: StatsRangeResponse = {
    range: 'today',
    label: 'Today',
    comparison_label: 'yesterday',
  };

  onMount(() => {
    const handleThemeChange = () => renderHourChart();
    const handlePointerDown = (event: PointerEvent) => {
      if (!rangeMenuOpen || !rangeMenuElement || !(event.target instanceof Node)) return;
      if (!rangeMenuElement.contains(event.target)) rangeMenuOpen = false;
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') rangeMenuOpen = false;
    };
    resizeObserver = new ResizeObserver(() => hourChart?.resize());
    window.addEventListener('spotrak:theme-change', handleThemeChange);
    document.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    void initialize();
    return () => {
      window.removeEventListener('spotrak:theme-change', handleThemeChange);
      document.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
      requestId += 1;
      destroyHourChart();
    };
  });

  async function initialize() {
    try {
      const me = await apiFetch<MeResponse>('/users/me');
      hourFormat = me.settings.hour_format ?? '24';
    } catch {
      hourFormat = '24';
    }
    await loadOverview();
  }

  async function loadOverview() {
    const request = ++requestId;
    destroyHourChart();
    loading = true;
    error = null;

    try {
      const overview = await apiFetch<OverviewStatsResponse>(overviewPath());

      if (request !== requestId) return;
      activeRange = overview.range;
      availableYears = overview.available_years.length > 0 ? overview.available_years : [currentYear];
      summary = overview.summary;
      previousSummary = overview.previous_summary ?? null;
      bestArtist = overview.best_artist ?? null;
      bestArtistStats = overview.best_artist_stats ?? null;
      bestSong = overview.best_song ?? null;
      hours = overview.hourly_distribution;
      history = overview.history;
    } catch (err) {
      if (request !== requestId) return;
      error = err instanceof Error ? err.message : 'Unable to load overview';
      summary = null;
      previousSummary = null;
      bestArtist = null;
      bestArtistStats = null;
      bestSong = null;
      hours = [];
      history = [];
    } finally {
      if (request === requestId) loading = false;
    }

    await tick();
    if (request === requestId) renderHourChart();
  }

  function overviewPath() {
    const params = new URLSearchParams({ range: rangeKey });
    if (rangeKey === 'selected-year') params.set('year', String(selectedYear));
    return `/stats/overview?${params.toString()}`;
  }

  function selectedRangeLabel() {
    if (rangeKey === 'selected-year') return String(selectedYear);
    const option = rangeButtons.find((item) => item.key === rangeKey);
    if (option?.key === 'all') return 'All time';
    return option?.label ?? activeRange.label;
  }

  function setRange(next: StatsRangeKey) {
    rangeKey = next;
    rangeMenuOpen = false;
    void loadOverview();
  }

  function chooseYear(year: number) {
    selectedYear = year;
    rangeKey = 'selected-year';
    rangeMenuOpen = false;
    void loadOverview();
  }

  function compareNumber(current: number | undefined, previous: number | undefined, label: string): Trend {
    if (previous === undefined || current === undefined) return null;
    if (previous === 0) {
      if (current === 0) return { text: `Same as ${label}`, tone: 'flat' };
      return { text: `No activity ${label}`, tone: 'up' };
    }
    const percent = ((current - previous) / previous) * 100;
    const abs = Math.abs(percent);
    const rounded = abs >= 10 ? Math.round(abs) : Math.round(abs * 10) / 10;
    if (rounded === 0) return { text: `Same as ${label}`, tone: 'flat' };
    return { text: `${rounded}% ${percent > 0 ? 'more' : 'less'} than ${label}`, tone: percent > 0 ? 'up' : 'down' };
  }

  function formatNumber(value: number | undefined) {
    return new Intl.NumberFormat().format(value ?? 0);
  }

  function formatMinutes(ms: number | undefined) {
    return `${formatNumber(Math.round((ms ?? 0) / 60_000))} min`;
  }

  function formatHour(hour: number) {
    if (hourFormat === '24') return `${String(hour).padStart(2, '0')}:00`;
    const suffix = hour < 12 ? 'AM' : 'PM';
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function historyTransition(event: HistoryEvent): string {
    return viewTransitionName(event.track_id, `overview-history-${event.id}`);
  }

  function songTransition(track: TopTrack): string {
    return viewTransitionName(track.id, `overview-best-song-${activeRange.range}-${track.id}`);
  }

  function artistTransition(artist: TopArtist): string {
    return viewTransitionName(artist.id, `overview-best-artist-${activeRange.range}-${artist.id}`);
  }

  function destroyHourChart() {
    resizeObserver?.disconnect();
    hourChart?.dispose();
    hourChart = null;
  }

  function renderHourChart() {
    if (!hourElement || loading || error) return;
    const colors = chartColors();
    const byHour = new Map(hours.map((point) => [point.hour, point]));
    const allHours = Array.from({ length: 24 }, (_, hour) => hour);
    hourChart ??= echarts.init(hourElement);
    resizeObserver?.observe(hourElement);
    hourChart.setOption({
      backgroundColor: 'transparent',
      color: [colors.primary],
      tooltip: {
        trigger: 'axis',
        ...chartTooltip(colors),
        formatter: (params: unknown) => hourTooltip(params),
      },
      grid: { left: 42, right: 14, top: 12, bottom: 34 },
      xAxis: {
        type: 'category',
        data: allHours.map(formatHour),
        axisLabel: { color: colors.muted, fontSize: 10, hideOverlap: true },
        axisLine: { lineStyle: { color: colors.border } },
        axisTick: { show: false },
      },
      yAxis: {
        type: 'value',
        splitLine: { lineStyle: { color: colors.border, opacity: 0.38 } },
        axisLabel: { color: colors.muted, fontSize: 10, formatter: (value: number) => formatChartValue(value, 'count') },
      },
      series: [
        {
          type: 'bar',
          data: allHours.map((hour) => byHour.get(hour)?.count ?? 0),
          barMaxWidth: 18,
          itemStyle: { borderRadius: [2, 2, 0, 0] },
        },
      ],
    });
  }

  function hourTooltip(params: unknown) {
    const first = Array.isArray(params) ? params[0] : params;
    const dataIndex = typeof first === 'object' && first && 'dataIndex' in first ? Number(first.dataIndex) : 0;
    const point = hours.find((item) => item.hour === dataIndex);
    return `
      <div class="overview-tooltip">
        <strong>${formatHour(dataIndex)}</strong>
        <span>${formatNumber(point?.count ?? 0)} plays</span>
        <span>${formatDuration(point?.duration_ms ?? 0)}</span>
      </div>
    `;
  }

</script>

<section class="overview-stack">
  <header class="overview-header">
    <div class="page-title">
      <h1>Overview</h1>
    </div>
    <div class="range-panel" aria-label="Overview time range">
      <div class="range-menu" bind:this={rangeMenuElement}>
        <Button
          variant="outline"
          size="sm"
          class="range-trigger"
          aria-haspopup="menu"
          aria-expanded={rangeMenuOpen}
          onclick={() => (rangeMenuOpen = !rangeMenuOpen)}
        >
          <span>{selectedRangeLabel()}</span>
          <ChevronDown class="range-trigger-icon" aria-hidden="true" />
        </Button>
        {#if rangeMenuOpen}
          <div class="range-dropdown" role="menu" aria-label="Choose overview time range">
            <div class="dropdown-group">
              {#each rangeButtons as option}
                <button type="button" role="menuitemradio" aria-checked={rangeKey === option.key} class:active-range={rangeKey === option.key} onclick={() => setRange(option.key)}>
                  <span>{option.key === 'all' ? 'All time' : option.label}</span>
                  {#if rangeKey === option.key}<Check aria-hidden="true" />{/if}
                </button>
              {/each}
            </div>
            <div class="dropdown-separator"></div>
            <span class="dropdown-label">Years</span>
            <div class="dropdown-group years">
              {#each availableYears as year}
                <button type="button" role="menuitemradio" aria-checked={rangeKey === 'selected-year' && selectedYear === year} class:active-range={rangeKey === 'selected-year' && selectedYear === year} onclick={() => chooseYear(year)}>
                  <span>{year}</span>
                  {#if rangeKey === 'selected-year' && selectedYear === year}<Check aria-hidden="true" />{/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if error}
    <Card.Root>
      <Card.Content><p class="error">{error}</p></Card.Content>
    </Card.Root>
  {/if}

  {#if loading}
    <div class="summary-grid">
      {#each Array(3) as _}
        <div class="skeleton loading-card"></div>
      {/each}
    </div>
    <div class="insights-grid">
      <div class="spotlight-stack">
        {#each Array(2) as _}
          <div class="skeleton loading-card"></div>
        {/each}
      </div>
      <div class="skeleton chart-loading"></div>
    </div>
  {:else if summary}
    <section class="summary-grid" aria-label={`${activeRange.label} summary`}>
      <Card.Root class="metric-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>Songs listened</Card.Title>
        </Card.Header>
        <Card.Content>
          <strong>{formatNumber(summary.total_listens)}</strong>
          {@const trend = compareNumber(summary.total_listens, previousSummary?.total_listens, activeRange.comparison_label ?? 'previous period')}
          {#if trend}<span class={`trend ${trend.tone}`}>{trend.text}</span>{/if}
        </Card.Content>
      </Card.Root>

      <Card.Root class="metric-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>Time listened</Card.Title>
        </Card.Header>
        <Card.Content>
          <strong>{formatDuration(summary.total_duration_ms)}</strong>
          {@const trend = compareNumber(summary.total_duration_ms, previousSummary?.total_duration_ms, activeRange.comparison_label ?? 'previous period')}
          {#if trend}<span class={`trend ${trend.tone}`}>{trend.text}</span>{/if}
        </Card.Content>
      </Card.Root>

      <Card.Root class="metric-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>Artists listened</Card.Title>
        </Card.Header>
        <Card.Content>
          <strong>{formatNumber(summary.unique_artists)}</strong>
          {@const trend = compareNumber(summary.unique_artists, previousSummary?.unique_artists, activeRange.comparison_label ?? 'previous period')}
          {#if trend}<span class={`trend ${trend.tone}`}>{trend.text}</span>{/if}
        </Card.Content>
      </Card.Root>

    </section>

    <section class="insights-grid" aria-label={`${activeRange.label} highlights and listening distribution`}>
      <div class="spotlight-stack" aria-label={`${activeRange.label} highlights`}>
        <Card.Root class="feature-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>Best artist</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if bestArtist}
            <div class="entity-row">
              <CoverArt src={bestArtist.image_url} name={bestArtist.name} href={transitionHref(`/artist/${bestArtist.id}`, artistTransition(bestArtist))} size="md" transitionName={artistTransition(bestArtist)} />
              <div class="entity-copy">
                <a class="entity-title" href={`/artist/${bestArtist.id}`}>{bestArtist.name}</a>
                <div class="stats-line">
                  <span>{formatNumber(bestArtist.count)} listens</span>
                  <span>{formatMinutes(bestArtist.duration_ms)}</span>
                  <span>{formatNumber(bestArtistStats?.unique_tracks)} different songs</span>
                </div>
              </div>
            </div>
          {:else}
            <p class="state">No artist for this range.</p>
          {/if}
        </Card.Content>
      </Card.Root>

        <Card.Root class="feature-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>Best song</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if bestSong}
            <div class="entity-row">
              <CoverArt src={bestSong.image_url} name={bestSong.name} href={transitionHref(`/track/${bestSong.id}`, songTransition(bestSong))} size="md" transitionName={songTransition(bestSong)} />
              <div class="entity-copy">
                <a class="entity-title" href={`/track/${bestSong.id}`}>{bestSong.name}</a>
                <span class="muted-line">{bestSong.artist_name} · {bestSong.album_name}</span>
                <div class="stats-line">
                  <span>{formatNumber(bestSong.count)} times</span>
                  <span>{formatMinutes(bestSong.duration_ms)}</span>
                </div>
              </div>
            </div>
          {:else}
            <p class="state">No song for this range.</p>
          {/if}
        </Card.Content>
        </Card.Root>
      </div>

      <Card.Root class="clock-card" size="sm">
      <Card.Header>
        <Card.Description>{hourFormat === '24' ? '24-hour format' : '12-hour format'}</Card.Description>
        <Card.Title>Listening distribution over the day</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if hours.length === 0}
          <p class="state">No hourly listening data for this range.</p>
        {:else}
          <div bind:this={hourElement} class="hour-chart" role="img" aria-label={`Listening distribution by local hour, ${hourFormat}-hour format`}></div>
        {/if}
      </Card.Content>
      </Card.Root>
    </section>

    <Card.Root class="history-card" size="sm">
      <Card.Header>
        <Card.Description>{activeRange.label}</Card.Description>
        <Card.Title>Listening history</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if history.length === 0}
          <p class="state">No history for this range.</p>
        {:else}
          <ol class="history-list">
            {#each history as event}
              <li>
                <CoverArt src={event.image_url} name={event.track_name} href={transitionHref(`/track/${event.track_id}`, historyTransition(event))} size="sm" transitionName={historyTransition(event)} />
                <div class="history-copy">
                  <a class="entity-title" href={`/track/${event.track_id}`}>{event.track_name}</a>
                  <span><a href={`/artist/${event.artist_id}`}>{event.artist_name}</a> · <a href={`/album/${event.album_id}`}>{event.album_name}</a></span>
                </div>
                <time datetime={event.played_at}>{formatDateTime(event.played_at)}</time>
                <small>{formatDuration(event.duration_ms)}</small>
              </li>
            {/each}
          </ol>
        {/if}
      </Card.Content>
    </Card.Root>
  {/if}
</section>

<style>
  .overview-stack {
    display: grid;
    gap: 0.75rem;
  }

  .overview-header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: end;
  }

  .range-panel {
    display: flex;
    justify-content: flex-end;
    align-items: center;
  }

  .range-menu {
    position: relative;
  }

  :global(.range-trigger) {
    min-width: 11.5rem;
    justify-content: space-between;
  }

  :global(.range-trigger-icon) {
    color: var(--color-muted);
  }

  .range-dropdown {
    position: absolute;
    top: calc(100% + 0.4rem);
    right: 0;
    z-index: 40;
    display: grid;
    gap: 0.18rem;
    width: 14rem;
    max-height: min(24rem, calc(100vh - 9rem));
    overflow: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.35rem;
    background: var(--color-bg-elevated);
    box-shadow: var(--shadow-card);
  }

  .dropdown-group {
    display: grid;
    gap: 0.1rem;
  }

  .range-dropdown button {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.55rem;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    text-align: left;
  }

  .range-dropdown button:hover,
  .range-dropdown button.active-range {
    background: var(--color-panel-2);
  }

  .range-dropdown button :global(svg) {
    width: 0.95rem;
    height: 0.95rem;
    color: var(--color-primary);
  }

  .dropdown-label {
    padding: 0.3rem 0.55rem 0.15rem;
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 850;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .dropdown-separator {
    height: 1px;
    margin: 0.28rem 0.15rem;
    background: var(--color-border);
  }

  .summary-grid,
  .insights-grid,
  .spotlight-stack {
    display: grid;
    gap: 0.6rem;
  }

  .summary-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .insights-grid {
    grid-template-columns: minmax(18rem, 0.75fr) minmax(26rem, 1.35fr);
    align-items: stretch;
  }

  :global(.metric-card [data-slot='card-content']) {
    display: grid;
    gap: 0.35rem;
  }

  :global(.metric-card strong) {
    color: var(--color-text);
    font-size: clamp(1.8rem, 4vw, 2.75rem);
    line-height: 0.9;
    letter-spacing: -0.08em;
  }

  .trend {
    color: var(--color-muted);
    font-size: 0.78rem;
    font-weight: 800;
  }

  .trend.up {
    color: var(--color-primary);
  }

  .trend.down {
    color: var(--color-danger);
  }

  :global(.feature-card [data-slot='card-content']) {
    display: flex;
    min-height: 4.9rem;
    align-items: center;
  }

  .entity-row {
    display: flex;
    gap: 0.7rem;
    align-items: center;
    min-width: 0;
  }

  .entity-copy,
  .history-copy {
    display: grid;
    min-width: 0;
    gap: 0.28rem;
  }

  .entity-title {
    overflow: hidden;
    color: var(--color-text);
    font-size: 1rem;
    font-weight: 850;
    line-height: 1.1;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entity-title:hover,
  .history-copy a:hover {
    color: var(--color-primary);
  }

  .stats-line {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.65rem;
    color: var(--color-muted);
    font-size: 0.78rem;
    font-weight: 780;
  }

  .muted-line,
  .state,
  .history-copy span,
  .history-list time,
  .history-list small {
    color: var(--color-muted);
    font-size: 0.8rem;
  }

  :global(.clock-card) {
    width: 100%;
    height: 100%;
  }

  .hour-chart {
    width: 100%;
    min-height: 15.75rem;
  }

  :global(.overview-tooltip) {
    display: grid;
    gap: 0.2rem;
  }

  :global(.overview-tooltip strong) {
    color: var(--color-text);
  }

  :global(.overview-tooltip span) {
    color: var(--color-muted);
  }

  .history-list {
    display: grid;
    gap: 0.2rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .history-list li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) minmax(10rem, auto) 5rem;
    gap: 0.7rem;
    align-items: center;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding: 0.45rem 0;
  }

  .history-copy span,
  .history-copy a {
    overflow: hidden;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-copy a {
    color: inherit;
  }

  .history-list time,
  .history-list small {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .loading-card {
    min-height: 7rem;
    border-radius: var(--radius-lg);
  }

  .chart-loading {
    min-height: 20rem;
    border-radius: var(--radius-lg);
  }

  .error {
    color: var(--color-danger);
  }

  @media (max-width: 900px) {
    .insights-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 740px) {
    .overview-header {
      align-items: stretch;
      flex-direction: column;
    }

    .range-panel {
      justify-content: flex-start;
    }

    .summary-grid {
      grid-template-columns: 1fr;
    }

    :global(.range-trigger) {
      width: 100%;
    }

    .range-menu,
    .range-panel {
      width: 100%;
    }

    .range-dropdown {
      right: auto;
      left: 0;
      width: min(100%, 18rem);
    }

    .history-list li {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .history-list time,
    .history-list small {
      display: none;
    }
  }

  @media (max-width: 420px) {
    .range-dropdown {
      width: 100%;
    }

    .entity-row {
      align-items: flex-start;
    }
  }
</style>
