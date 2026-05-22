<script lang="ts">
  import type { ComebackArtist } from "../../../lib/api/types";
  import { formatDate, formatGap, formatNumber } from "../../../lib/stats/insights";
  import CoverArt from "../../media/CoverArt.svelte";
  import * as Card from "../../ui/card";

  export let artists: ComebackArtist[] = [];
  export let pagePrefix = "";
  export let timezone: string | null = null;
  export let className = "";

  function artistHref(id: string): string {
    return `${pagePrefix}/artist/${id}`;
  }
</script>

<Card.Root class={`stats-card comebacks-card ${className}`}>
  <Card.Header>
    <Card.Description>longest gaps before a return</Card.Description>
    <Card.Title>Comeback artists</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if artists.length === 0}
      <p class="state">No comeback artists for this range.</p>
    {:else}
      <ol class="comeback-list">
        {#each artists as artist, index (artist.artist_id)}
          <li>
            <span class="rank">{String(index + 1).padStart(2, "0")}</span>
            <CoverArt
              src={artist.image_url}
              name={artist.artist_name}
              href={artistHref(artist.artist_id)}
              size="xs"
            />
            <a class="artist-copy" href={artistHref(artist.artist_id)}>
              <strong>{artist.artist_name}</strong>
              <small>returned {formatDate(artist.returned_at, timezone)}</small>
            </a>
            <span class="gap">
              <strong>{formatGap(artist.gap_ms)}</strong>
              <small>{formatNumber(artist.range_listens)} listens</small>
            </span>
          </li>
        {/each}
      </ol>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.comebacks-card) {
    min-width: 0;
    height: 100%;
  }

  .comeback-list {
    display: grid;
    gap: 0.5rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .comeback-list li {
    display: grid;
    grid-template-columns: 1.55rem auto minmax(0, 1fr) auto;
    gap: 0.55rem;
    align-items: center;
    min-width: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 62%, transparent);
    padding-bottom: 0.5rem;
  }

  .comeback-list li:last-child {
    border-bottom: 0;
    padding-bottom: 0;
  }

  .rank {
    color: color-mix(in srgb, var(--color-muted) 70%, transparent);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    font-weight: 800;
  }

  .artist-copy,
  .gap {
    display: grid;
    gap: 0.13rem;
    min-width: 0;
    text-decoration: none;
  }

  .artist-copy strong,
  .artist-copy small,
  .gap strong,
  .gap small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist-copy strong {
    color: var(--color-text);
    font-size: 0.86rem;
    line-height: 1.08;
  }

  .artist-copy small,
  .gap small {
    color: var(--color-muted);
    font-size: 0.68rem;
  }

  .gap {
    justify-items: end;
    text-align: right;
  }

  .gap strong {
    color: var(--color-text);
    font-size: 1rem;
    font-weight: 820;
    font-variant-numeric: tabular-nums;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 520px) {
    .comeback-list li {
      grid-template-columns: 1.35rem auto minmax(0, 1fr);
    }

    .gap {
      grid-column: 3;
      justify-items: start;
      text-align: left;
    }
  }
</style>
