<script lang="ts">
  import type { ArtistRunSummary, RepeatLoopStats, RepeatLoopSummary } from "../../../lib/api/types";
  import { formatDuration } from "../../../lib/date/format";
  import { formatNumber } from "../../../lib/stats/insights";
  import CoverArt from "../../media/CoverArt.svelte";
  import * as Card from "../../ui/card";

  export let loops: RepeatLoopStats | null = null;
  export let pagePrefix = "";
  export let className = "";

  $: hasData = Boolean(
    loops?.top_track_loop ||
      loops?.back_to_back_track_run ||
      loops?.longest_artist_run ||
      (loops?.total_back_to_back_repeats ?? 0) > 0,
  );

  function trackHref(track: RepeatLoopSummary): string {
    return `${pagePrefix}/track/${track.track_id}`;
  }

  function artistHref(run: ArtistRunSummary): string {
    return `${pagePrefix}/artist/${run.artist_id}`;
  }
</script>

<Card.Root class={`stats-card repeats-card ${className}`}>
  <Card.Header>
    <Card.Description>close-range replays and runs</Card.Description>
    <Card.Title>Repeat loops</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No repeat loops for this range.</p>
    {:else}
      <div class="repeat-summary">
        <span>Back-to-back repeats</span>
        <strong>{formatNumber(loops?.total_back_to_back_repeats)}</strong>
      </div>

      <div class="loop-picks">
        {#if loops?.top_track_loop}
          <a class="loop-row" href={trackHref(loops.top_track_loop)}>
            <CoverArt src={loops.top_track_loop.image_url} name={loops.top_track_loop.track_name} size="xs" />
            <span>
              <small>Replay cluster</small>
              <strong>{loops.top_track_loop.track_name}</strong>
              <em>{loops.top_track_loop.artist_name}</em>
            </span>
            <b>{loops.top_track_loop.listens}x</b>
          </a>
        {/if}

        {#if loops?.back_to_back_track_run}
          <a class="loop-row" href={trackHref(loops.back_to_back_track_run)}>
            <CoverArt src={loops.back_to_back_track_run.image_url} name={loops.back_to_back_track_run.track_name} size="xs" />
            <span>
              <small>Longest same-track run</small>
              <strong>{loops.back_to_back_track_run.track_name}</strong>
              <em>{formatDuration(loops.back_to_back_track_run.listening_duration_ms)}</em>
            </span>
            <b>{loops.back_to_back_track_run.listens}x</b>
          </a>
        {/if}

        {#if loops?.longest_artist_run}
          <a class="loop-row" href={artistHref(loops.longest_artist_run)}>
            <CoverArt src={loops.longest_artist_run.image_url} name={loops.longest_artist_run.artist_name} size="xs" />
            <span>
              <small>Longest same-artist run</small>
              <strong>{loops.longest_artist_run.artist_name}</strong>
              <em>{formatDuration(loops.longest_artist_run.listening_duration_ms)}</em>
            </span>
            <b>{loops.longest_artist_run.listens}x</b>
          </a>
        {/if}
      </div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.repeats-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.repeats-card [data-slot="card-content"]) {
    display: grid;
    gap: 0.75rem;
  }

  .repeat-summary {
    display: grid;
    gap: 0.15rem;
  }

  .repeat-summary span,
  .loop-row small {
    color: var(--color-muted);
    font-size: 0.68rem;
    font-style: normal;
    font-weight: 780;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .repeat-summary strong {
    color: var(--color-text);
    font-size: clamp(1.9rem, 4vw, 2.7rem);
    line-height: 0.95;
    font-variant-numeric: tabular-nums;
  }

  .loop-picks {
    display: grid;
    gap: 0.5rem;
  }

  .loop-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.58rem;
    align-items: center;
    min-width: 0;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding-top: 0.55rem;
    color: var(--color-text);
    text-decoration: none;
  }

  .loop-row span {
    display: grid;
    gap: 0.12rem;
    min-width: 0;
  }

  .loop-row strong,
  .loop-row em {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .loop-row strong {
    color: var(--color-text);
    font-size: 0.86rem;
    font-style: normal;
    line-height: 1.08;
  }

  .loop-row em {
    color: var(--color-muted);
    font-size: 0.7rem;
    font-style: normal;
  }

  .loop-row b {
    color: var(--color-text);
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
  }

  .loop-row:hover strong,
  .loop-row:focus-visible strong {
    color: var(--color-primary);
  }

  .loop-row:focus-visible {
    border-radius: var(--radius-xs);
    outline: 2px solid color-mix(in srgb, var(--color-primary) 36%, transparent);
    outline-offset: 3px;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }
</style>
