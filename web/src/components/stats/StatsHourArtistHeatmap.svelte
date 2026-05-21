<script lang="ts">
  import type { HourRepartitionPoint, HourlyTopArtist } from '../../lib/api/types';
  import { formatCountValue } from '../../lib/charts/theme';
  import { formatDuration } from '../../lib/date/format';
  import * as Card from '../ui/card';

  type HourSlot = {
    hour: number;
    label: string;
    totalListens: number;
    totalDurationMs: number;
    artist: HourlyTopArtist | null;
    intensity: number;
  };

  export let artists: HourlyTopArtist[] = [];
  export let hours: HourRepartitionPoint[] = [];
  export let hourFormat: '12' | '24' = '24';
  export let pagePrefix = '';
  export let className = '';

  $: hourTotals = new Map(hours.map((point) => [point.hour, point]));
  $: artistsByHour = new Map(artists.map((artist) => [artist.hour, artist]));
  $: maxTotal = Math.max(1, ...hours.map((point) => point.count));
  $: slots = Array.from({ length: 24 }, (_, hour): HourSlot => {
    const total = hourTotals.get(hour);
    const artist = artistsByHour.get(hour) ?? null;
    return {
      hour,
      label: formatHour(hour),
      totalListens: total?.count ?? 0,
      totalDurationMs: total?.duration_ms ?? 0,
      artist,
      intensity: ((total?.count ?? 0) / maxTotal) * 100,
    };
  });
  $: hasData = hours.length > 0 || artists.length > 0;

  function formatHour(hour: number): string {
    if (hourFormat === '24') return `${String(hour).padStart(2, '0')}:00`;
    const suffix = hour < 12 ? 'AM' : 'PM';
    const value = hour % 12 || 12;
    return `${value} ${suffix}`;
  }

  function artistHref(id: string): string {
    return `${pagePrefix}/artist/${id}`;
  }

  function slotTitle(slot: HourSlot): string {
    if (!slot.artist) return `${slot.label}: no listens`;
    return `${slot.label}: ${slot.artist.artist_name}, ${formatCountValue(slot.artist.count)} listens. ${formatCountValue(slot.totalListens)} total listens, ${formatDuration(slot.totalDurationMs)}.`;
  }

  function slotAriaLabel(slot: HourSlot): string {
    if (!slot.artist) return `${slot.label}: no listens`;
    return `${slot.label}: top artist ${slot.artist.artist_name}, ${slot.artist.count.toLocaleString()} listens. ${slot.totalListens.toLocaleString()} total listens.`;
  }

  function heatFill(slot: HourSlot): number {
    return Math.round(8 + slot.intensity * 0.45);
  }

  function heatFillSoft(slot: HourSlot): number {
    return Math.round(5 + slot.intensity * 0.28);
  }

  function heatBorder(slot: HourSlot): number {
    return Math.round(18 + slot.intensity * 0.38);
  }
</script>

<Card.Root class={`hour-artist-card ${className} overflow-visible`}>
  <Card.Header>
    <Card.Description>intensity is total listens in that local hour</Card.Description>
    <Card.Title>Top artist by hour</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No hourly artist data for this range.</p>
    {:else}
      <div class="hour-heatmap" role="group" aria-label="Top artist for each local hour">
        {#each slots as slot (slot.hour)}
          {#if slot.artist}
            <a
              class="hour-cell"
              href={artistHref(slot.artist.artist_id)}
              style={`--fill-strength: ${heatFill(slot)}%; --fill-soft-strength: ${heatFillSoft(slot)}%; --border-strength: ${heatBorder(slot)}%;`}
              title={slotTitle(slot)}
              data-tooltip={slotTitle(slot)}
              aria-label={slotAriaLabel(slot)}
            >
              <span class="hour-label">{slot.label}</span>
              <strong>{slot.artist.artist_name}</strong>
              <small>{slot.artist.count.toLocaleString()} listens</small>
            </a>
          {:else}
            <div
              class="hour-cell empty"
              style="--fill-strength: 0%; --fill-soft-strength: 0%; --border-strength: 0%;"
              title={slotTitle(slot)}
              data-tooltip={slotTitle(slot)}
              aria-label={slotAriaLabel(slot)}
            >
              <span class="hour-label">{slot.label}</span>
              <strong>No listens</strong>
              <small>0 listens</small>
            </div>
          {/if}
        {/each}
      </div>
      <table class="sr-only">
        <caption>Top artist by local hour</caption>
        <thead>
          <tr><th scope="col">Hour</th><th scope="col">Artist</th><th scope="col">Artist listens</th><th scope="col">Total listens</th><th scope="col">Time listened</th></tr>
        </thead>
        <tbody>
          {#each slots as slot (slot.hour)}
            <tr>
              <td>{slot.label}</td>
              <td>{slot.artist?.artist_name ?? 'No listens'}</td>
              <td>{slot.artist?.count ?? 0}</td>
              <td>{slot.totalListens}</td>
              <td>{formatDuration(slot.totalDurationMs)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.hour-artist-card) {
    min-width: 0;
    height: 100%;
  }

  .hour-heatmap {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0.38rem;
  }

  .hour-cell {
    --tooltip-left: 50%;
    --tooltip-right: auto;
    --tooltip-x: -50%;

    position: relative;
    display: grid;
    gap: 0.2rem;
    min-height: 5.1rem;
    min-width: 0;
    border: 1px solid color-mix(in srgb, var(--chart-1) var(--border-strength), var(--color-border));
    border-radius: var(--radius-sm);
    padding: 0.52rem;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--chart-1) var(--fill-strength), var(--color-bg-elevated)),
        color-mix(in srgb, var(--chart-1) var(--fill-soft-strength), var(--color-panel))
      );
    color: var(--color-text);
    text-decoration: none;
  }

  a.hour-cell:hover,
  a.hour-cell:focus-visible {
    border-color: color-mix(in srgb, var(--chart-1) 72%, var(--color-border));
    outline: none;
    transform: translateY(-1px);
  }

  .hour-cell.empty {
    border-color: color-mix(in srgb, var(--color-border) 72%, transparent);
    background: color-mix(in srgb, var(--color-panel) 70%, transparent);
    color: var(--color-muted);
  }

  .hour-cell::after {
    position: absolute;
    z-index: 3;
    bottom: calc(100% + 0.45rem);
    left: var(--tooltip-left);
    right: var(--tooltip-right);
    width: max-content;
    max-width: min(18rem, 80vw);
    border: 1px solid color-mix(in srgb, var(--color-border) 62%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.45rem 0.55rem;
    background: var(--color-bg-elevated);
    box-shadow: var(--shadow-card);
    color: var(--color-text);
    content: attr(data-tooltip);
    font-size: 0.72rem;
    line-height: 1.25;
    opacity: 0;
    pointer-events: none;
    transform: translate(var(--tooltip-x), 0.2rem);
    transition: opacity 160ms ease, transform 160ms ease;
    white-space: normal;
  }

  .hour-cell:hover::after,
  .hour-cell:focus-visible::after {
    opacity: 1;
    transform: translate(var(--tooltip-x), 0);
  }

  .hour-cell:nth-child(6n+1) {
    --tooltip-left: 0;
    --tooltip-right: auto;
    --tooltip-x: 0;
  }

  .hour-cell:nth-child(6n) {
    --tooltip-left: auto;
    --tooltip-right: 0;
    --tooltip-x: 0;
  }

  .hour-label,
  .hour-cell small {
    color: var(--color-muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.67rem;
    font-variant-numeric: tabular-nums;
    font-weight: 800;
  }

  .hour-cell strong {
    overflow: hidden;
    font-size: 0.76rem;
    line-height: 1.05;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hour-cell small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 720px) {
    .hour-heatmap {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .hour-cell {
      --tooltip-left: 50%;
      --tooltip-right: auto;
      --tooltip-x: -50%;
    }

    .hour-cell:nth-child(3n+1) {
      --tooltip-left: 0;
      --tooltip-right: auto;
      --tooltip-x: 0;
    }

    .hour-cell:nth-child(3n) {
      --tooltip-left: auto;
      --tooltip-right: 0;
      --tooltip-x: 0;
    }
  }

  @media (max-width: 420px) {
    .hour-heatmap {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .hour-cell {
      --tooltip-left: 50%;
      --tooltip-right: auto;
      --tooltip-x: -50%;
    }

    .hour-cell:nth-child(2n+1) {
      --tooltip-left: 0;
      --tooltip-right: auto;
      --tooltip-x: 0;
    }

    .hour-cell:nth-child(2n) {
      --tooltip-left: auto;
      --tooltip-right: 0;
      --tooltip-x: 0;
    }
  }
</style>
