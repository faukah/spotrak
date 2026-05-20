<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { SpotifyCurrentlyPlaying } from '../../lib/api/types';
  import { formatDuration } from '../../lib/date/format';
  import { spotifyImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';

  let current: SpotifyCurrentlyPlaying | null = null;
  let loading = true;
  let error: string | null = null;

  $: track = current?.item ?? null;
  $: transition = track ? viewTransitionName(track.id, 'currently-playing') : undefined;

  onMount(() => {
    void load();
    const interval = window.setInterval(() => void load(false), 30_000);
    return () => window.clearInterval(interval);
  });

  async function load(showLoading = true) {
    loading = showLoading;
    try {
      current = await apiFetch<SpotifyCurrentlyPlaying | null>('/spotify/currently-playing', { cache: 'no-store' });
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load current playback';
    } finally {
      loading = false;
    }
  }
</script>

<Card.Root class="currently-card">
  <Card.Content>
    {#if loading}
      <div class="state">Loading current playback…</div>
    {:else if error}
      <div class="state error">{error}</div>
    {:else if track}
      <div class="now">
        <CoverArt src={spotifyImageUrl(track.images) ?? spotifyImageUrl(track.album.images)} name={track.name} href={transitionHref(`/track/${track.id}`, transition ?? '')} size="sm" transitionName={transition} />
        <div>
          <p class="kicker">{current?.is_playing ? 'Currently playing' : 'Paused'}</p>
          <a href={`/track/${track.id}`}><strong>{track.name}</strong></a>
          <span>{track.artists.map((artist) => artist.name).join(', ')} · {track.album.name}</span>
        </div>
        <small>{current?.progress_ms ? `${formatDuration(current.progress_ms)} / ` : ''}{formatDuration(track.duration_ms)}</small>
      </div>
    {:else}
      <div class="state">Nothing playing right now.</div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.currently-card) {
    border-color: color-mix(in srgb, var(--color-primary) 28%, var(--color-border));
  }

  .now {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.7rem;
    align-items: center;
  }

  .now div {
    display: grid;
    gap: 0.12rem;
    min-width: 0;
  }

  p,
  strong,
  span {
    overflow: hidden;
    margin: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  a {
    color: inherit;
    text-decoration: none;
  }

  span,
  small,
  .state {
    color: var(--color-muted);
    font-size: 0.82rem;
  }

  .error {
    color: var(--color-danger);
  }

  @media (max-width: 700px) {
    .now {
      grid-template-columns: auto minmax(0, 1fr);
    }
    small { display: none; }
  }
</style>
