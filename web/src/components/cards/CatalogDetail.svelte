<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { AlbumDetail, ArtistDetail, EntityStats, TrackDetail } from '../../lib/api/types';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { spotifyImageUrl, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';

  export let kind: 'track' | 'artist' | 'album';
  export let id: string;

  type Detail = TrackDetail | ArtistDetail | AlbumDetail;
  export let initialDetail: Detail | null = null;
  export let transitionName: string | undefined = undefined;
  export let apiPrefix = '';

  let detail: Detail | null = initialDetail;
  let stats: EntityStats | null = null;
  let detailLoading = !initialDetail;
  let statsLoading = true;
  let error: string | null = null;
  let refreshTimer: number | undefined;

  $: plural = `${kind}s`;
  $: image = detail ? imageFor(detail) : null;
  $: activeTransitionName = transitionName ?? (detail ? viewTransitionName(detail.id) : undefined);

  onMount(() => {
    cleanTransitionUrl();
    if (detail) {
      maybeRefreshHydratedArtist(detail);
    } else {
      void loadDetail(true);
    }
    void loadStats();
    return () => {
      if (refreshTimer) window.clearTimeout(refreshTimer);
    };
  });

  function cleanTransitionUrl() {
    if (typeof window === 'undefined') return;
    const url = new URL(window.location.href);
    if (!url.searchParams.has('vt')) return;
    url.searchParams.delete('vt');
    window.history.replaceState(window.history.state, '', `${url.pathname}${url.search}${url.hash}`);
  }

  async function loadDetail(scheduleHydrationRefresh = false) {
    try {
      detail = await apiFetch<Detail>(`${apiPrefix}/${plural}/${id}`);
      if (scheduleHydrationRefresh) maybeRefreshHydratedArtist(detail);
    } catch (err) {
      error = err instanceof Error ? err.message : `Unable to load ${kind}`;
    } finally {
      detailLoading = false;
    }
  }

  function maybeRefreshHydratedArtist(value: Detail) {
    if (kind === 'artist' && !imageFor(value)) {
      refreshTimer = window.setTimeout(() => void loadDetail(false), 8_000);
    }
  }

  async function loadStats() {
    try {
      stats = await apiFetch<EntityStats>(`${apiPrefix}/${plural}/${id}/stats`);
    } catch {
      // Detail is the primary content; stats can remain absent if this request fails.
    } finally {
      statsLoading = false;
    }
  }

  function subtitle(value: Detail): string {
    if ('album' in value) return `${value.artists.map((artist) => artist.name).join(', ')} · ${value.album.name}`;
    if ('artists' in value) return value.artists.map((artist) => artist.name).join(', ');
    if ('genres' in value && Array.isArray(value.genres)) return value.genres.join(', ');
    return '';
  }

  function imageFor(value: Detail): string | null {
    if ('album' in value) return spotifyImageUrl(value.images) ?? spotifyImageUrl(value.album.images);
    return spotifyImageUrl(value.images);
  }
</script>

{#if detailLoading}
  <div class="detail-skeleton skeleton"></div>
{:else if error}
  <Card.Root><Card.Content><p class="error">{error}</p></Card.Content></Card.Root>
{:else if detail}
  <section class="detail-header">
    <CoverArt src={image} name={detail.name} size="xl" transitionName={activeTransitionName} />
    <div class="detail-copy">
      <p class="kicker">{kind}</p>
      <h1>{detail.name}</h1>
      {#if subtitle(detail)}<p class="muted subtitle">{subtitle(detail)}</p>{/if}
      {#if 'href' in detail && detail.href}
        <a class="spotify-link" href={detail.href} target="_blank" rel="noreferrer">Open in Spotify</a>
      {/if}
    </div>
  </section>

  {#if stats}
    <div class="stat-grid">
      <Card.Root size="sm"><Card.Header><Card.Description>Listens</Card.Description><Card.Title>{stats.total_listens.toLocaleString()}</Card.Title></Card.Header></Card.Root>
      <Card.Root size="sm"><Card.Header><Card.Description>Duration</Card.Description><Card.Title>{formatDuration(stats.total_duration_ms)}</Card.Title></Card.Header></Card.Root>
      <Card.Root size="sm"><Card.Header><Card.Description>First played</Card.Description><Card.Title>{stats.first_played_at ? formatDateTime(stats.first_played_at) : 'Never'}</Card.Title></Card.Header></Card.Root>
      <Card.Root size="sm"><Card.Header><Card.Description>Last played</Card.Description><Card.Title>{stats.last_played_at ? formatDateTime(stats.last_played_at) : 'Never'}</Card.Title></Card.Header></Card.Root>
    </div>
  {:else if statsLoading}
    <div class="stat-grid">
      <div class="stat-skeleton skeleton"></div>
      <div class="stat-skeleton skeleton"></div>
      <div class="stat-skeleton skeleton"></div>
      <div class="stat-skeleton skeleton"></div>
    </div>
  {/if}
{/if}

<style>
  .detail-header {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: clamp(1rem, 3vw, 2rem);
    align-items: end;
    margin-bottom: 1rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: clamp(0.75rem, 2vw, 1rem);
    background: var(--color-panel);
    box-shadow: var(--shadow-card);
  }

  .detail-copy {
    display: grid;
    gap: 0.55rem;
    min-width: 0;
    padding-bottom: 0.5rem;
  }

  h1 {
    overflow-wrap: anywhere;
  }

  .subtitle {
    margin: 0;
    font-size: 1rem;
  }

  .spotify-link {
    width: max-content;
    border-bottom: 1px solid var(--color-primary);
    color: var(--color-primary);
    font-size: 0.86rem;
    font-weight: 700;
    text-decoration: none;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .detail-skeleton,
  .stat-skeleton {
    border-radius: var(--radius-lg);
  }

  .detail-skeleton {
    min-height: 26rem;
  }

  .stat-skeleton {
    min-height: 5rem;
  }

  .error {
    color: var(--color-danger);
  }

  @media (max-width: 760px) {
    .detail-header,
    .stat-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
