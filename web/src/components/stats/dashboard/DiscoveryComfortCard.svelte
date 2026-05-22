<script lang="ts">
  import type { DiscoveryStats } from "../../../lib/api/types";
  import { formatNumber, formatPercent } from "../../../lib/stats/insights";
  import * as Card from "../../ui/card";

  export let stats: DiscoveryStats | null = null;
  export let className = "";

  $: totalListens = stats?.total_listens ?? 0;
  $: discoveryShare = clampShare(stats?.discovery_share ?? 0);
  $: repeatShare = clampShare(stats?.repeat_share ?? 0);
  $: hasData = totalListens > 0;
  $: newTrackShare = stats && stats.unique_tracks > 0
    ? (stats.new_tracks / stats.unique_tracks) * 100
    : 0;
  $: newArtistShare = stats && stats.unique_artists > 0
    ? (stats.new_artists / stats.unique_artists) * 100
    : 0;

  function clampShare(value: number): number {
    if (!Number.isFinite(value)) return 0;
    return Math.max(0, Math.min(100, value));
  }
</script>

<Card.Root class={`stats-card discovery-card ${className}`}>
  <Card.Header>
    <Card.Description>new-to-you vs repeats</Card.Description>
    <Card.Title>Discovery vs comfort</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No listening data for this range.</p>
    {:else}
      <div class="split-meter" aria-label={`${formatPercent(discoveryShare)} discovery, ${formatPercent(repeatShare)} repeat listening`}>
        <span class="discovery" style={`--share: ${discoveryShare}%;`}></span>
        <span class="comfort"></span>
      </div>

      <div class="headline">
        <strong>{formatPercent(discoveryShare)}</strong>
        <span>of listens were first plays in this range</span>
      </div>

      <dl class="insight-grid">
        <div>
          <dt>New tracks</dt>
          <dd>{formatNumber(stats?.new_tracks)}</dd>
          <small>{formatPercent(newTrackShare)} of unique tracks</small>
        </div>
        <div>
          <dt>New artists</dt>
          <dd>{formatNumber(stats?.new_artists)}</dd>
          <small>{formatPercent(newArtistShare)} of unique artists</small>
        </div>
        <div>
          <dt>Repeat listens</dt>
          <dd>{formatNumber(stats?.repeat_listens)}</dd>
          <small>{formatPercent(repeatShare)} comfort share</small>
        </div>
      </dl>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.discovery-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.discovery-card [data-slot="card-content"]) {
    display: grid;
    gap: 0.9rem;
  }

  .split-meter {
    display: flex;
    height: 0.55rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 54%, transparent);
  }

  .split-meter .discovery {
    flex: 0 0 var(--share);
    min-width: 0.25rem;
    border-radius: inherit;
    background: var(--chart-3);
  }

  .split-meter .comfort {
    flex: 1 1 auto;
    min-width: 0.25rem;
    background: color-mix(in srgb, var(--color-muted) 48%, transparent);
  }

  .headline {
    display: grid;
    gap: 0.18rem;
  }

  .headline strong {
    color: var(--color-text);
    font-size: clamp(2rem, 5vw, 3.4rem);
    line-height: 0.95;
  }

  .headline span {
    max-width: 24ch;
    color: var(--color-muted);
    font-size: 0.82rem;
    line-height: 1.35;
  }

  .insight-grid {
    display: grid;
    gap: 0.5rem;
    margin: 0;
  }

  .insight-grid div {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.1rem 0.75rem;
    align-items: baseline;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding-top: 0.52rem;
  }

  .insight-grid dt {
    color: var(--color-muted);
    font-size: 0.7rem;
    font-weight: 760;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .insight-grid dd {
    margin: 0;
    color: var(--color-text);
    font-size: 1.02rem;
    font-weight: 820;
    font-variant-numeric: tabular-nums;
  }

  .insight-grid small {
    grid-column: 1 / -1;
    color: color-mix(in srgb, var(--color-muted) 84%, transparent);
    font-size: 0.72rem;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }
</style>
