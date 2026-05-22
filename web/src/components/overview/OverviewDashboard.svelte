<script lang="ts">
  import { ArrowRight, Import as ImportIcon, RefreshCw, Settings } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { apiFetch } from '../../lib/api/client';
  import type {
    EntityStats,
    HistoryEvent,
    OverviewStatsResponse,
    StatsRangeKey,
    StatsRangeResponse,
    SummaryStats,
    TopArtist,
    TopTrack,
  } from '../../lib/api/types';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { transitionHref, viewTransitionName } from '../../lib/images';
  import { selectedStatsRange, statsRangeSelectionKey, type StatsRangeSelection } from '../../lib/stores/stats-range';
  import CurrentlyPlaying from '../layout/CurrentlyPlaying.svelte';
  import CoverArt from '../media/CoverArt.svelte';
  import StatsRangePicker from '../stats/StatsRangePicker.svelte';
  import { Button } from '../ui/button';
  import * as Card from '../ui/card';

  type ChangeTone = 'up' | 'down' | 'flat' | 'new';
  type ChangeItem = {
    key: string;
    label: string;
    value: string;
    detail: string;
    tone: ChangeTone;
    score: number;
  };

  export let initialOverview: OverviewStatsResponse | null = null;
  export let apiPrefix = '';
  export let pagePrefix = '';

  const currentYear = new Date().getFullYear();

  let rangeKey: StatsRangeKey = initialOverview?.range.range ?? 'all';
  let selectedYear = currentYear;
  let availableYears: number[] = initialOverview?.available_years.length ? initialOverview.available_years : [currentYear];
  let timezone = initialOverview?.timezone ?? null;

  let summary: SummaryStats | null = initialOverview?.summary ?? null;
  let previousSummary: SummaryStats | null = initialOverview?.previous_summary ?? null;
  let bestArtist: TopArtist | null = initialOverview?.best_artist ?? null;
  let bestArtistStats: EntityStats | null = initialOverview?.best_artist_stats ?? null;
  let bestSong: TopTrack | null = initialOverview?.best_song ?? null;
  let history: HistoryEvent[] = initialOverview?.history ?? [];

  let loading = initialOverview === null;
  let refreshing = false;
  let error: string | null = null;
  let requestId = 0;

  let unsubscribeStatsRange: (() => void) | undefined;
  let lastRangeSelectionKey = statsRangeSelectionKey({ range: rangeKey, year: selectedYear });
  let activeRange: StatsRangeResponse = initialOverview?.range ?? {
    range: 'all',
    label: 'All time',
    comparison_label: null,
  };
  $: comparisonLabel = activeRange.comparison_label ?? 'previous period';
  $: changeItems = summary
    ? [
        buildChangeItem('listens', 'Listens', summary.total_listens, previousSummary?.total_listens, formatNumber(summary.total_listens)),
        buildChangeItem(
          'duration',
          'Time listened',
          summary.total_duration_ms,
          previousSummary?.total_duration_ms,
          formatDuration(summary.total_duration_ms),
        ),
        buildChangeItem(
          'artists',
          'Unique artists',
          summary.unique_artists,
          previousSummary?.unique_artists,
          formatNumber(summary.unique_artists),
        ),
        buildChangeItem(
          'tracks',
          'Unique tracks',
          summary.unique_tracks,
          previousSummary?.unique_tracks,
          formatNumber(summary.unique_tracks),
        ),
        buildChangeItem(
          'albums',
          'Unique albums',
          summary.unique_albums,
          previousSummary?.unique_albums,
          formatNumber(summary.unique_albums),
        ),
      ]
    : [];
  $: primaryChange = strongestChange(changeItems);
  $: secondaryChangeItems = primaryChange ? changeItems.filter((item) => item.key !== primaryChange.key) : changeItems;

  onMount(() => {
    applyStatsRangeSelection(get(selectedStatsRange), false);
    unsubscribeStatsRange = selectedStatsRange.subscribe((selection) => {
      const key = statsRangeSelectionKey(selection);
      if (key === lastRangeSelectionKey) return;
      applyStatsRangeSelection(selection, true);
    });
    void initialize();
    return () => {
      unsubscribeStatsRange?.();
      requestId += 1;
    };
  });

  async function initialize() {
    if (initialOverview && activeOverviewPath() === overviewPath()) {
      loading = false;
      refreshing = false;
      if (rangeKey === 'today') void prefetchOverview('week');
      return;
    }

    await loadOverview();
    if (rangeKey === 'today') void prefetchOverview('week');
  }

  function applyStatsRangeSelection(selection: StatsRangeSelection, shouldLoad: boolean) {
    rangeKey = selection.range;
    if (selection.range === 'selected-year') selectedYear = selection.year ?? selectedYear;
    lastRangeSelectionKey = statsRangeSelectionKey({ range: rangeKey, year: selectedYear });
    if (!shouldLoad && summary !== null && activeOverviewPath() !== overviewPath()) {
      clearOverviewData();
      loading = true;
    }
    if (shouldLoad) void loadOverview();
  }

  async function loadOverview() {
    const request = ++requestId;
    const path = overviewPath();

    loading = summary === null;
    refreshing = summary !== null;
    error = null;

    try {
      const overview = await apiFetch<OverviewStatsResponse>(path);

      if (request !== requestId) return;
      applyOverview(overview);
    } catch (err) {
      if (request !== requestId) return;
      error = err instanceof Error ? err.message : 'Unable to load overview';
      if (summary === null) {
        previousSummary = null;
        bestArtist = null;
        bestArtistStats = null;
        bestSong = null;
        history = [];
      }
    } finally {
      if (request === requestId) {
        loading = false;
        refreshing = false;
      }
    }
  }

  async function prefetchOverview(range: StatsRangeKey) {
    try {
      await apiFetch<OverviewStatsResponse>(overviewPathFor(range, selectedYear));
    } catch {
      return;
    }
  }

  function applyOverview(overview: OverviewStatsResponse) {
    activeRange = overview.range;
    rangeKey = overview.range.range;
    if (overview.range.range === 'selected-year') selectedYear = overviewYear(overview.range);
    lastRangeSelectionKey = statsRangeSelectionKey({ range: rangeKey, year: selectedYear });
    availableYears = overview.available_years.length > 0 ? overview.available_years : [currentYear];
    summary = overview.summary;
    previousSummary = overview.previous_summary ?? null;
    bestArtist = overview.best_artist ?? null;
    bestArtistStats = overview.best_artist_stats ?? null;
    bestSong = overview.best_song ?? null;
    history = overview.history;
    timezone = overview.timezone;
  }

  function clearOverviewData() {
    error = null;
    summary = null;
    previousSummary = null;
    bestArtist = null;
    bestArtistStats = null;
    bestSong = null;
    history = [];
    refreshing = false;
  }

  function overviewPath() {
    return overviewPathFor(rangeKey, selectedYear);
  }

  function activeOverviewPath() {
    return overviewPathFor(activeRange.range, overviewYear(activeRange));
  }

  function overviewPathFor(range: StatsRangeKey, year: number) {
    const params = new URLSearchParams({ range });
    if (range === 'selected-year') params.set('year', String(year));
    return `${apiPrefix}/stats/overview?${params.toString()}`;
  }

  function overviewYear(range: StatsRangeResponse): number {
    if (range.range !== 'selected-year') return currentYear;
    const year = Number(range.label);
    return Number.isInteger(year) ? year : currentYear;
  }

  function buildChangeItem(key: string, label: string, current: number, previous: number | undefined, value: string): ChangeItem {
    if (previous === undefined) {
      return {
        key,
        label,
        value,
        detail: 'No comparison window',
        tone: 'flat',
        score: 0,
      };
    }

    if (previous === 0) {
      if (current === 0) {
        return { key, label, value, detail: `Same as ${comparisonLabel}`, tone: 'flat', score: 0 };
      }
      return {
        key,
        label,
        value,
        detail: `New activity vs ${comparisonLabel}`,
        tone: 'new',
        score: Number.MAX_SAFE_INTEGER,
      };
    }

    const percent = ((current - previous) / previous) * 100;
    const rounded = roundedPercent(percent);
    const direction = percent > 0 ? 'more' : 'less';
    const sign = percent > 0 ? '+' : '-';

    if (rounded === 0) {
      return { key, label, value, detail: `Same as ${comparisonLabel}`, tone: 'flat', score: 0 };
    }

    return {
      key,
      label,
      value,
      detail: `${sign}${rounded}% ${direction} than ${comparisonLabel}`,
      tone: percent > 0 ? 'up' : 'down',
      score: Math.abs(percent),
    };
  }

  function roundedPercent(percent: number): number {
    const abs = Math.abs(percent);
    return abs >= 10 ? Math.round(abs) : Math.round(abs * 10) / 10;
  }

  function strongestChange(items: ChangeItem[]): ChangeItem | null {
    if (items.length === 0) return null;
    return items.toSorted((a, b) => b.score - a.score)[0] ?? null;
  }

  function formatNumber(value: number | undefined) {
    return new Intl.NumberFormat().format(value ?? 0);
  }

  function formatMinutes(ms: number | undefined) {
    return `${formatNumber(Math.round((ms ?? 0) / 60_000))} min`;
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

  function pageHref(path: string): string {
    return `${pagePrefix}${path}`;
  }
</script>

<section class="overview-stack" aria-busy={refreshing}>
  <header class="overview-header">
    <div class="page-title">
      <h1>Overview</h1>
    </div>
    <div class="range-panel" aria-label="Overview time range">
      <StatsRangePicker {availableYears} ariaLabel="Choose overview time range" />
    </div>
  </header>

  {#if error}
    {#if summary === null}
      <Card.Root class="diagnostic-card" size="sm">
        <Card.Header>
          <Card.Description>Overview unavailable</Card.Description>
          <Card.Title>Refresh failed</Card.Title>
        </Card.Header>
        <Card.Content>
          <p class="error">{error}</p>
          <div class="action-row">
            <Button size="sm" onclick={() => void loadOverview()}>
              <RefreshCw data-icon="inline-start" aria-hidden="true" />
              Retry
            </Button>
            <Button variant="outline" size="sm" href={pageHref('/imports')}>
              <ImportIcon data-icon="inline-start" aria-hidden="true" />
              Imports
            </Button>
            <Button variant="outline" size="sm" href={pageHref('/settings')}>
              <Settings data-icon="inline-start" aria-hidden="true" />
              Settings
            </Button>
          </div>
        </Card.Content>
      </Card.Root>
    {:else}
      <div class="inline-error" role="alert">
        <span>Refresh failed: {error}</span>
        <Button variant="outline" size="xs" onclick={() => void loadOverview()}>
          <RefreshCw data-icon="inline-start" aria-hidden="true" />
          Retry
        </Button>
      </div>
    {/if}
  {/if}

  {#if loading}
    <div class="command-grid">
      <div class="skeleton loading-card large"></div>
      <div class="skeleton loading-card"></div>
    </div>
    <div class="entity-grid">
      {#each Array(2) as _}
        <div class="skeleton loading-card"></div>
      {/each}
    </div>
    <div class="skeleton history-loading"></div>
  {:else if summary}
    <section class="command-grid" aria-label={`${activeRange.label} command center`}>
      <Card.Root class="change-card" size="sm">
        <Card.Header>
          <Card.Description>{activeRange.label}</Card.Description>
          <Card.Title>What changed</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if primaryChange}
            <div class={`primary-change ${primaryChange.tone}`}>
              <span class="change-eyebrow">Largest movement</span>
              <strong>{primaryChange.label}</strong>
              <span>{primaryChange.value}</span>
              <small>{primaryChange.detail}</small>
            </div>
          {/if}
          <div class="change-list" aria-label={`${activeRange.label} changes compared with ${comparisonLabel}`}>
            {#each secondaryChangeItems as item (item.key)}
              <div class={`change-row ${item.tone}`}>
                <span>{item.label}</span>
                <strong>{item.value}</strong>
                <small>{item.detail}</small>
              </div>
            {/each}
          </div>
        </Card.Content>
      </Card.Root>

      <CurrentlyPlaying
        endpoint={`${apiPrefix}/player/currently-playing`}
        variant="card"
        {pagePrefix}
        showReconnect={apiPrefix === ''}
      />
    </section>

    <section class="entity-grid" aria-label={`${activeRange.label} leading music`}>
        <Card.Root class="feature-card" size="sm">
        <Card.Header>
          <Card.Description>Dominant artist</Card.Description>
          <Card.Title>Top artist</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if bestArtist}
            <div class="entity-row">
              <CoverArt src={bestArtist.image_url} name={bestArtist.name} href={transitionHref(pageHref(`/artist/${bestArtist.id}`), artistTransition(bestArtist))} size="lg" transitionName={artistTransition(bestArtist)} />
              <div class="entity-copy">
                <a class="entity-title" href={pageHref(`/artist/${bestArtist.id}`)}>{bestArtist.name}</a>
                <div class="stats-line">
                  <span>{formatNumber(bestArtist.count)} listens</span>
                  <span>{formatMinutes(bestArtist.duration_ms)}</span>
                  <span>{formatNumber(bestArtistStats?.unique_tracks)} tracks</span>
                </div>
              </div>
            </div>
          {:else}
            <div class="state-block">
              <p class="state">No top artist for this range.</p>
              <Button variant="outline" size="xs" href={pageHref('/imports')}>
                <ImportIcon data-icon="inline-start" aria-hidden="true" />
                Check imports
              </Button>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>

        <Card.Root class="feature-card" size="sm">
        <Card.Header>
          <Card.Description>Most repeated track</Card.Description>
          <Card.Title>Top track</Card.Title>
        </Card.Header>
        <Card.Content>
          {#if bestSong}
            <div class="entity-row">
              <CoverArt src={bestSong.image_url} name={bestSong.name} href={transitionHref(pageHref(`/track/${bestSong.id}`), songTransition(bestSong))} size="lg" transitionName={songTransition(bestSong)} />
              <div class="entity-copy">
                <a class="entity-title" href={pageHref(`/track/${bestSong.id}`)}>{bestSong.name}</a>
                <span class="muted-line">{bestSong.artist_name} · {bestSong.album_name}</span>
                <div class="stats-line">
                  <span>{formatNumber(bestSong.count)} listens</span>
                  <span>{formatMinutes(bestSong.duration_ms)}</span>
                </div>
              </div>
            </div>
          {:else}
            <div class="state-block">
              <p class="state">No top track for this range.</p>
              <Button variant="outline" size="xs" href={pageHref('/imports')}>
                <ImportIcon data-icon="inline-start" aria-hidden="true" />
                Check imports
              </Button>
            </div>
          {/if}
        </Card.Content>
        </Card.Root>
    </section>

    <section class="history-section" aria-label={`${activeRange.label} latest plays`}>
      <Card.Root class="history-card" size="sm">
        <Card.Header>
          <Card.Description>Latest plays</Card.Description>
          <Card.Title>Listening history</Card.Title>
          <Card.Action>
            <Button variant="outline" size="xs" href={pageHref('/history')}>
              Full history
              <ArrowRight data-icon="inline-end" aria-hidden="true" />
            </Button>
          </Card.Action>
        </Card.Header>
        <Card.Content>
          {#if history.length === 0}
            <div class="state-block">
              <p class="state">No history for this range.</p>
              <div class="action-row">
                <Button variant="outline" size="xs" onclick={() => void loadOverview()}>
                  <RefreshCw data-icon="inline-start" aria-hidden="true" />
                  Retry
                </Button>
                <Button variant="outline" size="xs" href={pageHref('/imports')}>
                  <ImportIcon data-icon="inline-start" aria-hidden="true" />
                  Imports
                </Button>
              </div>
            </div>
          {:else}
            <ol class="history-list">
              {#each history as event (event.id)}
                <li>
                  <CoverArt src={event.image_url} name={event.track_name} href={transitionHref(pageHref(`/track/${event.track_id}`), historyTransition(event))} size="sm" transitionName={historyTransition(event)} />
                  <div class="history-copy">
                    <a class="entity-title" href={pageHref(`/track/${event.track_id}`)}>{event.track_name}</a>
                    <span><a href={pageHref(`/artist/${event.artist_id}`)}>{event.artist_name}</a> · <a href={pageHref(`/album/${event.album_id}`)}>{event.album_name}</a></span>
                  </div>
                  <time datetime={event.played_at}>{formatDateTime(event.played_at, timezone)}</time>
                  <small>{formatDuration(event.duration_ms)}</small>
                </li>
              {/each}
            </ol>
          {/if}
        </Card.Content>
      </Card.Root>
    </section>
  {/if}
</section>

<style>
  .overview-stack {
    display: grid;
    gap: 0.65rem;
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

  .command-grid,
  .entity-grid {
    display: grid;
    gap: 0.6rem;
  }

  .command-grid {
    grid-template-columns: minmax(22rem, 1.35fr) minmax(18rem, 0.85fr);
    align-items: stretch;
  }

  .entity-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: stretch;
  }

  :global(.change-card),
  :global(.feature-card),
  :global(.history-card),
  :global(.diagnostic-card) {
    gap: 0.85rem;
    padding-block: 1rem;
  }

  :global(.change-card [data-slot='card-header']),
  :global(.change-card [data-slot='card-content']),
  :global(.feature-card [data-slot='card-header']),
  :global(.feature-card [data-slot='card-content']),
  :global(.history-card [data-slot='card-header']),
  :global(.history-card [data-slot='card-content']),
  :global(.diagnostic-card [data-slot='card-header']),
  :global(.diagnostic-card [data-slot='card-content']) {
    padding-inline: 1rem;
  }

  :global(.change-card [data-slot='card-content']) {
    display: grid;
    gap: 0.75rem;
  }

  .primary-change {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.18rem 0.7rem;
    align-items: end;
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
    padding: 0.75rem;
    background: color-mix(in srgb, var(--color-panel) 78%, transparent);
  }

  .primary-change .change-eyebrow,
  .change-row span,
  .history-list time,
  .history-list small {
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .primary-change .change-eyebrow {
    grid-column: 1 / -1;
  }

  .primary-change strong {
    color: var(--color-text);
    font-size: clamp(1.2rem, 2vw, 1.55rem);
    line-height: 1;
  }

  .primary-change > span:not(.change-eyebrow) {
    color: var(--color-text);
    font-size: clamp(1.2rem, 2vw, 1.55rem);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .primary-change small {
    grid-column: 1 / -1;
    color: var(--color-muted);
    font-size: 0.8rem;
    font-weight: 760;
  }

  .primary-change.up small,
  .primary-change.new small,
  .change-row.up small,
  .change-row.new small {
    color: var(--color-primary);
  }

  .primary-change.down small,
  .change-row.down small {
    color: var(--color-danger);
  }

  .change-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8.5rem, 1fr));
    gap: 1px;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-border) 72%, transparent);
  }

  .change-row {
    display: grid;
    gap: 0.14rem;
    min-width: 0;
    padding: 0.55rem 0.6rem;
    background: var(--color-bg-elevated);
  }

  .change-row strong {
    overflow: hidden;
    color: var(--color-text);
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
    line-height: 1.05;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .change-row small {
    overflow: hidden;
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }

  .inline-error {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    justify-content: space-between;
    border: 1px solid color-mix(in srgb, var(--color-danger) 35%, var(--color-border));
    border-radius: var(--radius-md);
    padding: 0.55rem 0.65rem;
    background: color-mix(in srgb, var(--color-danger) 8%, var(--color-bg-elevated));
    color: var(--color-danger);
    font-size: 0.82rem;
    font-weight: 700;
  }

  :global(.feature-card [data-slot='card-content']) {
    display: flex;
    min-height: 7.75rem;
    align-items: center;
  }

  .entity-row {
    display: flex;
    gap: 0.9rem;
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

  :global(.feature-card) .entity-title {
    font-size: clamp(1.08rem, 1.6vw, 1.32rem);
  }

  :global(.feature-card) .stats-line,
  :global(.feature-card) .muted-line {
    font-size: 0.86rem;
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

  .state {
    margin: 0;
  }

  .state-block {
    display: grid;
    gap: 0.55rem;
    align-items: start;
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

  .loading-card.large {
    min-height: 11rem;
  }

  .history-loading {
    min-height: 22rem;
    border-radius: var(--radius-lg);
  }

  .error {
    margin: 0;
    color: var(--color-danger);
  }

  @media (max-width: 1180px) {
    .command-grid {
      grid-template-columns: 1fr;
    }

    .change-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
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

    .entity-grid,
    .change-list {
      grid-template-columns: 1fr;
    }

    .range-panel {
      width: 100%;
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
    .entity-row {
      align-items: flex-start;
    }
  }
</style>
