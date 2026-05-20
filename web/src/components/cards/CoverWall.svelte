<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { TopTrack } from '../../lib/api/types';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import { formatDuration } from '../../lib/date/format';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';

  export let limit = 12;
  export let compact = false;

  let tracks: TopTrack[] = [];
  let loading = true;
  let error: string | null = null;

  $: lead = tracks[0];
  $: rest = tracks.slice(1, limit);

  function coverTransition(track: TopTrack, index: number): string {
    return viewTransitionName(track.id, `cover-wall-${compact ? 'compact' : 'full'}-${limit}-${index}`);
  }

  function coverHref(track: TopTrack, index: number): string {
    return transitionHref(`/track/${track.id}`, coverTransition(track, index));
  }

  onMount(async () => {
    try {
      tracks = await apiFetch<TopTrack[]>(`/stats/top/tracks?limit=${limit}&metric=count`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load covers';
    } finally {
      loading = false;
    }
  });
</script>

<Card.Root class="cover-wall-card" data-compact={compact}>
  <Card.Header class="cover-wall-header">
    <div>
      <Card.Title>Cover wall</Card.Title>
    </div>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="wall-skeleton skeleton"></div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if tracks.length === 0}
      <p class="state">No covers yet. Poll Spotify or import privacy data.</p>
    {:else}
      <div class="wall">
        {#if lead}
          <a class="lead" href={coverHref(lead, 0)}>
            <CoverArt src={directImageUrl(lead)} name={lead.name} size="xl" transitionName={coverTransition(lead, 0)} />
            <div class="lead-copy">
              <span>most played</span>
              <strong>{lead.name}</strong>
              <small>{lead.artist_name} · {lead.count.toLocaleString()} plays · {formatDuration(lead.duration_ms)}</small>
            </div>
          </a>
        {/if}
        <div class="tiles">
          {#each rest as track, index}
            <a class="tile" href={coverHref(track, index + 1)} title={`${track.name} · ${track.artist_name}`}>
              <CoverArt src={directImageUrl(track)} name={track.name} size="lg" transitionName={coverTransition(track, index + 1)} />
              <span>{track.name}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.cover-wall-card) {
    overflow: hidden;
  }

  .wall {
    display: grid;
    grid-template-columns: minmax(16rem, 0.9fr) minmax(18rem, 1.1fr);
    gap: 0.75rem;
  }

  .lead {
    position: relative;
    min-height: 24rem;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-panel);
    color: var(--color-text);
    text-decoration: none;
  }

  .lead :global(.cover) {
    width: 100%;
    height: 100%;
    border: 0;
    border-radius: 0;
  }

  .lead-copy {
    position: absolute;
    inset: auto 0 0;
    display: grid;
    gap: 0.3rem;
    padding: 1rem;
    background: rgb(0 0 0 / 0.82);
  }

  .lead-copy span,
  .lead-copy small,
  .state {
    color: var(--color-muted);
  }

  .lead-copy span {
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .lead-copy strong {
    font-size: clamp(1.8rem, 4vw, 3.1rem);
    line-height: 0.9;
    letter-spacing: -0.08em;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
  }

  .tile {
    position: relative;
    overflow: hidden;
    color: var(--color-text);
    text-decoration: none;
  }

  .tile :global(.cover) {
    width: 100%;
  }

  .tile span {
    position: absolute;
    inset: auto 0 0;
    overflow: hidden;
    padding: 1.4rem 0.5rem 0.45rem;
    background: rgb(0 0 0 / 0.78);
    color: color-mix(in srgb, var(--color-text) 92%, transparent);
    font-size: 0.72rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wall-skeleton {
    min-height: 24rem;
    border-radius: var(--radius-sm);
  }

  .error {
    color: var(--color-danger);
  }

  :global(.cover-wall-card[data-compact='true']) .wall {
    grid-template-columns: 1fr;
  }

  :global(.cover-wall-card[data-compact='true']) .lead {
    min-height: 16rem;
  }

  :global(.cover-wall-card[data-compact='true']) .tiles {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  @media (max-width: 900px) {
    .wall {
      grid-template-columns: 1fr;
    }

    .lead {
      min-height: 18rem;
    }
  }

  @media (max-width: 540px) {
    .tiles {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
