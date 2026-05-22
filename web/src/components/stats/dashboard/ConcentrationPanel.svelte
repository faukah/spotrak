<script lang="ts">
  import type { ListeningConcentrationStats } from "../../../lib/api/types";
  import { formatNumber, formatPercent } from "../../../lib/stats/insights";
  import CoverArt from "../../media/CoverArt.svelte";
  import * as Card from "../../ui/card";

  export let concentration: ListeningConcentrationStats | null = null;
  export let pagePrefix = "";
  export let className = "";

  $: hasData = (concentration?.total_listens ?? 0) > 0;
  $: effectiveCount = concentration?.effective_artist_count ?? 0;
  $: topArtistShare = clampShare(concentration?.top_artist_share ?? 0);
  $: nextFourShare = clampShare((concentration?.top_five_share ?? 0) - topArtistShare);
  $: nextFiveShare = clampShare((concentration?.top_ten_share ?? 0) - (concentration?.top_five_share ?? 0));
  $: restShare = clampShare(100 - topArtistShare - nextFourShare - nextFiveShare);
  $: breadthShare = concentration && concentration.artist_count > 0
    ? (effectiveCount / concentration.artist_count) * 100
    : 0;

  function clampShare(value: number): number {
    if (!Number.isFinite(value)) return 0;
    return Math.max(0, Math.min(100, value));
  }

  function artistHref(id: string | null | undefined): string | undefined {
    return id ? `${pagePrefix}/artist/${id}` : undefined;
  }
</script>

<Card.Root class={`stats-card concentration-card ${className}`}>
  <Card.Header>
    <Card.Description>how concentrated the range was</Card.Description>
    <Card.Title>Obsession index</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No concentration data for this range.</p>
    {:else}
      <div class="lead-artist">
        <CoverArt
          src={concentration?.top_artist_image_url}
          name={concentration?.top_artist_name ?? "Top artist"}
          href={artistHref(concentration?.top_artist_id)}
          size="sm"
        />
        <div>
          <span>Top artist share</span>
          <strong>{formatPercent(concentration?.top_artist_share, 1)}</strong>
          <p>{concentration?.top_artist_name ?? "No artist"}</p>
        </div>
      </div>

      <div class="share-stack" aria-label={`Top artist ${formatPercent(topArtistShare, 1)}, artists 2 through 5 ${formatPercent(nextFourShare, 1)}, artists 6 through 10 ${formatPercent(nextFiveShare, 1)}, everyone else ${formatPercent(restShare, 1)}`}>
        <span class="top" style={`--share: ${topArtistShare}%;`}></span>
        <span class="five" style={`--share: ${nextFourShare}%;`}></span>
        <span class="ten" style={`--share: ${nextFiveShare}%;`}></span>
        <span class="rest"></span>
      </div>

      <dl class="concentration-grid">
        <div>
          <dt>Top 5</dt>
          <dd>{formatPercent(concentration?.top_five_share, 1)}</dd>
        </div>
        <div>
          <dt>Top 10</dt>
          <dd>{formatPercent(concentration?.top_ten_share, 1)}</dd>
        </div>
        <div>
          <dt>Effective artists</dt>
          <dd>{formatNumber(effectiveCount, 1)}</dd>
          <small>{formatPercent(breadthShare)} of {formatNumber(concentration?.artist_count)} artists</small>
        </div>
      </dl>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.concentration-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.concentration-card [data-slot="card-content"]) {
    display: grid;
    gap: 0.85rem;
  }

  .lead-artist {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.68rem;
    align-items: center;
    min-width: 0;
  }

  .lead-artist div {
    display: grid;
    gap: 0.16rem;
    min-width: 0;
  }

  .lead-artist span,
  .concentration-grid dt {
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 780;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .lead-artist strong {
    color: var(--color-text);
    font-size: clamp(1.8rem, 4vw, 2.6rem);
    line-height: 0.95;
    font-variant-numeric: tabular-nums;
  }

  .lead-artist p {
    overflow: hidden;
    margin: 0;
    color: var(--color-text);
    font-weight: 780;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .share-stack {
    display: flex;
    height: 0.55rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 54%, transparent);
  }

  .share-stack span {
    min-width: 0;
  }

  .share-stack .top {
    flex: 0 0 var(--share);
    background: var(--chart-1);
  }

  .share-stack .five {
    flex: 0 0 var(--share);
    background: var(--chart-2);
  }

  .share-stack .ten {
    flex: 0 0 var(--share);
    background: var(--chart-3);
  }

  .share-stack .rest {
    flex: 1 1 auto;
    min-width: 0.25rem;
    background: color-mix(in srgb, var(--color-muted) 34%, transparent);
  }

  .concentration-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.5rem;
    margin: 0;
  }

  .concentration-grid div {
    display: grid;
    gap: 0.14rem;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding-top: 0.55rem;
  }

  .concentration-grid dd {
    margin: 0;
    color: var(--color-text);
    font-size: 1.05rem;
    font-weight: 820;
    font-variant-numeric: tabular-nums;
  }

  .concentration-grid small {
    overflow: hidden;
    color: var(--color-muted);
    font-size: 0.7rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 520px) {
    .concentration-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
