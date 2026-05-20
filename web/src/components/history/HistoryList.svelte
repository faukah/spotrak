<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { HistoryEvent, MeResponse } from '../../lib/api/types';
  import { formatDateTime, formatDuration } from '../../lib/date/format';
  import { transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';

  export let limit = 25;
  export let apiPrefix = '';
  export let pagePrefix = '';

  let events: HistoryEvent[] = [];
  let loading = true;
  let error: string | null = null;
  let timezone: string | null = null;

  onMount(() => {
    void load();
  });

  function coverTransition(event: HistoryEvent): string {
    return viewTransitionName(event.track_id, `history-${event.id}`);
  }

  async function load() {
    loading = true;
    try {
      if (!apiPrefix) {
        const me = await apiFetch<MeResponse>('/users/me');
        timezone = me.settings.timezone ?? null;
      }
      events = await apiFetch<HistoryEvent[]>(`${apiPrefix}/history?limit=${limit}`);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load history';
    } finally {
      loading = false;
    }
  }
</script>

<Card.Root class="history-card">
  <Card.Header>
    <Card.Description>{limit} latest plays</Card.Description>
    <Card.Title>Recent listening history</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="rows" aria-live="polite">
        {#each Array(Math.min(limit, 12)) as _}<div class="skeleton"></div>{/each}
      </div>
    {:else if error}
      <p class="state error">{error}</p>
    {:else if events.length === 0}
      <p class="state">No plays yet. Polling or imports will populate your history.</p>
    {:else}
      <ol class="rows">
        {#each events as event (event.id)}
          <li>
            <CoverArt src={event.image_url} name={event.track_name} href={transitionHref(`${pagePrefix}/track/${event.track_id}`, coverTransition(event))} size="sm" transitionName={coverTransition(event)} />
            <div class="track">
              <a class="title-link" href={`${pagePrefix}/track/${event.track_id}`}><strong>{event.track_name}</strong></a>
              <span><a href={`${pagePrefix}/artist/${event.artist_id}`}>{event.artist_name}</a> · <a href={`${pagePrefix}/album/${event.album_id}`}>{event.album_name}</a></span>
            </div>
            <time datetime={event.played_at}>{formatDateTime(event.played_at, timezone)}</time>
            <small>{formatDuration(event.duration_ms)}</small>
          </li>
        {/each}
      </ol>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  .rows {
    display: grid;
    gap: 0.2rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) minmax(10rem, auto) 5rem;
    gap: 0.7rem;
    align-items: center;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding: 0.42rem 0;
  }

  .track {
    display: grid;
    min-width: 0;
    color: var(--color-text);
  }

  .title-link {
    min-width: 0;
    color: inherit;
    text-decoration: none;
  }

  strong,
  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  strong {
    font-size: 0.95rem;
  }

  span,
  time,
  small,
  .state {
    color: var(--color-muted);
    font-size: 0.78rem;
  }

  span a {
    color: inherit;
    text-decoration: none;
  }

  span a:hover {
    color: var(--color-text);
  }

  time,
  small {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .error {
    color: var(--color-danger);
  }

  .skeleton {
    height: 3.4rem;
    border-radius: var(--radius-sm);
  }

  @media (max-width: 760px) {
    li {
      grid-template-columns: auto minmax(0, 1fr);
    }

    time,
    small {
      display: none;
    }
  }
</style>
