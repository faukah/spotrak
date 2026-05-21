<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { apiFetch } from '../../lib/api/client';
  import type { TimelinePoint, TopAlbum, TopArtist, TopTrack } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import { selectedStatsMetric } from '../../lib/stores/preferences';
  import { selectedStatsRange, statsRangeQuery, statsRangeSelectionKey, type StatsRangeSelection } from '../../lib/stores/stats-range';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import StatsRangePicker from '../stats/StatsRangePicker.svelte';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';

  export let kind: 'tracks' | 'artists' | 'albums' = 'tracks';
  export let limit = 10;
  export let dense = false;
  export let apiPrefix = '';
  export let pagePrefix = '';
  export let showRangePicker = true;

  type Item = TopTrack | TopArtist | TopAlbum;
  let items: Item[] = [];
  let availableYears: number[] = [new Date().getFullYear()];
  let loading = true;
  let error: string | null = null;
  let activeMetric: 'count' | 'duration' = 'count';
  let activeRange: StatsRangeSelection = { range: 'all' };
  let unsubscribeMetric: (() => void) | undefined;
  let unsubscribeRange: (() => void) | undefined;
  const metrics = [
    { value: 'count' as const, label: 'Plays' },
    { value: 'duration' as const, label: 'Time' },
  ]; 

  $: title = `Top ${kind}`;
  $: moreHref = `${pagePrefix}/top/${kind}`;

  onMount(() => {
    activeMetric = get(selectedStatsMetric);
    activeRange = get(selectedStatsRange);
    void load();
    if (showRangePicker) void loadAvailableYears();

    unsubscribeMetric = selectedStatsMetric.subscribe((metric) => {
      if (metric === activeMetric) return;
      activeMetric = metric;
      void load();
    });
    unsubscribeRange = selectedStatsRange.subscribe((selection) => {
      if (statsRangeSelectionKey(selection) === statsRangeSelectionKey(activeRange)) return;
      activeRange = selection;
      void load();
    });

    return () => {
      unsubscribeMetric?.();
      unsubscribeRange?.();
    };
  });

  async function load() {
    loading = true;
    error = null;
    try {
      const params = new URLSearchParams(statsRangeQuery(activeRange));
      params.set('limit', String(limit));
      params.set('metric', activeMetric);
      items = await apiFetch<Item[]>(`${apiPrefix}/stats/top/${kind}?${params.toString()}`);
    } catch (err) {
      error = err instanceof Error ? err.message : `Unable to load top ${kind}`;
    } finally {
      loading = false;
    }
  }

  async function loadAvailableYears() {
    try {
      const timeline = await apiFetch<TimelinePoint[]>(`${apiPrefix}/stats/listening-over-time?split=year`);
      const years = timeline
        .map((point) => Number(point.bucket.slice(0, 4)))
        .filter((year) => Number.isInteger(year));
      availableYears = Array.from(new Set([...availableYears, ...years])).toSorted((a, b) => b - a);
    } catch {
      availableYears = [...availableYears];
    }
  }

  function subtitle(item: Item): string {
    if ('artist_name' in item && item.artist_name && 'album_name' in item) return `${item.artist_name} · ${item.album_name}`;
    if ('artist_name' in item && item.artist_name) return item.artist_name;
    if ('release_year' in item && item.release_year) return String(item.release_year);
    return '';
  }

  function href(item: Item): string {
    if (kind === 'tracks') return `${pagePrefix}/track/${item.id}`;
    if (kind === 'artists') return `${pagePrefix}/artist/${item.id}`;
    return `${pagePrefix}/album/${item.id}`;
  }

  function coverTransition(item: Item, index: number): string {
    return viewTransitionName(item.id, `top-${kind}-${dense ? 'dense' : 'full'}-${limit}-${index}`);
  }

  function coverHref(item: Item, index: number): string {
    return transitionHref(href(item), coverTransition(item, index));
  }

  function metricLabel(item: Item): string {
    return activeMetric === 'duration' ? formatDuration(item.duration_ms) : `${item.count.toLocaleString()} plays`;
  }

  function setMetric(metric: 'count' | 'duration') {
    selectedStatsMetric.set(metric);
  }
</script>

<Card.Root class="top-list-card" data-dense={dense}>
  <Card.Header class="top-list-header">
    <div>
      <Card.Title>{title}</Card.Title>
    </div>
    <div class="header-actions">
      {#if showRangePicker}
        <StatsRangePicker {availableYears} ariaLabel={`Choose top ${kind} time range`} buttonSize={dense ? 'xs' : 'sm'} />
      {/if}
      <div class="metric-buttons" aria-label="Ranking metric">
        {#each metrics as metric (metric.value)}
          <Button variant={activeMetric === metric.value ? 'default' : 'outline'} size="xs" onclick={() => setMetric(metric.value)}>{metric.label}</Button>
        {/each}
      </div>
      {#if limit < 20}<Button href={moreHref} variant="ghost" size="sm">All</Button>{/if}
    </div>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="rows" aria-live="polite">
        {#each Array(Math.min(limit, 8)) as _, index}
          <div class="row skeleton" style={`--delay: ${index * 50}ms`}></div>
        {/each}
      </div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if items.length === 0}
      <p class="state">No {kind} yet.</p>
    {:else}
      <ol class="rows">
        {#each items as item, index (item.id)}
          <li class:leader={index === 0 && !dense}>
            <span class="rank">{String(index + 1).padStart(2, '0')}</span>
            <a class="cover-link" href={coverHref(item, index)} aria-label={item.name}>
              <CoverArt src={directImageUrl(item)} name={item.name} size={index === 0 && !dense ? 'lg' : 'sm'} transitionName={coverTransition(item, index)} />
            </a>
            <a class="item-title" href={href(item)}>
              <strong>{item.name}</strong>
              {#if subtitle(item)}<small>{subtitle(item)}</small>{/if}
            </a>
            <span class="metric">{metricLabel(item)}</span>
          </li>
        {/each}
      </ol>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.top-list-card) {
    height: 100%;
  }

  :global(.top-list-header) {
    display: flex;
    flex-direction: row;
    align-items: start;
    justify-content: space-between;
    gap: 1rem;
  }

  .header-actions,
  .metric-buttons {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }

  .header-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .rows {
    display: grid;
    gap: 0.45rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 2.2rem auto minmax(0, 1fr) auto;
    gap: 0.7rem;
    align-items: center;
    border: 1px solid transparent;
    border-bottom-color: color-mix(in srgb, var(--color-border) 72%, transparent);
    padding: 0.45rem 0;
    animation: row-in 260ms ease both;
  }

  li.leader {
    grid-template-columns: 2.4rem auto minmax(0, 1fr);
    grid-template-areas:
      'rank cover title'
      'rank cover metric';
    align-items: end;
    margin-bottom: 0.35rem;
    border: 1px solid color-mix(in srgb, var(--color-border) 85%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.65rem;
    background: var(--color-panel-2);
  }

  li.leader .rank { grid-area: rank; align-self: start; }
  li.leader .cover-link { grid-area: cover; }
  li.leader .item-title { grid-area: title; }
  li.leader .metric { grid-area: metric; justify-self: start; margin-bottom: 0.4rem; }

  .cover-link {
    display: block;
    width: max-content;
    color: inherit;
    text-decoration: none;
  }

  .rank {
    color: color-mix(in srgb, var(--color-muted) 70%, transparent);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: -0.05em;
  }

  .item-title {
    display: grid;
    gap: 0.08rem;
    min-width: 0;
    color: var(--color-text);
    text-decoration: none;
  }

  strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.95rem;
    line-height: 1.2;
  }

  li.leader strong {
    font-size: clamp(1.25rem, 2.8vw, 2rem);
    letter-spacing: -0.06em;
    white-space: normal;
  }

  small,
  .metric,
  .state {
    color: var(--color-muted);
  }

  small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.78rem;
  }

  .metric {
    font-size: 0.78rem;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .error {
    color: var(--color-danger);
  }

  .skeleton {
    height: 3.4rem;
    border-radius: var(--radius-sm);
    animation-delay: var(--delay);
  }
  @keyframes row-in { from { opacity: 0; transform: translateY(6px); } }

  @media (max-width: 620px) {
    :global(.top-list-header) {
      align-items: stretch;
      flex-direction: column;
    }

    .header-actions {
      justify-content: flex-start;
    }

    li,
    li.leader {
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-areas: none;
      align-items: center;
      margin: 0;
      border: 0;
      border-bottom: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
      border-radius: 0;
      padding: 0.45rem 0;
      background: transparent;
    }

    li.leader .rank,
    li.leader .cover-link,
    li.leader .item-title,
    li.leader .metric {
      grid-area: auto;
      align-self: center;
      justify-self: auto;
      margin: 0;
    }

    li.leader .cover-link :global(.cover) {
      --cover-size: 3rem;
    }

    li.leader strong {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 0.95rem;
      letter-spacing: 0;
    }

    .rank,
    .metric {
      display: none;
    }
  }
</style>
