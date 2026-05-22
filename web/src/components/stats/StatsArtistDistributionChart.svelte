<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { BucketedTopArtist } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatPercentValue } from '../../lib/charts/theme';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import {
    buildDistributionBuckets,
    buildDistributionEntities,
    buildDistributionLanes,
    type DistributionEntity,
    type DistributionLane,
    type LanePoint,
  } from '../../lib/stats/artist-distribution';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';

  export let rows: BucketedTopArtist[] = [];
  export let bucketKeys: string[] = [];
  export let timelineDescription = 'monthly buckets';
  export let pagePrefix = '';
  export let formatBucketLabel: (bucket: string) => string = (bucket) => bucket;
  export let formatBucketPreposition: (bucket: string) => string = () => 'in';
  export let className = '';

  const DESKTOP_INITIAL_LANES = 24;
  const COMPACT_INITIAL_LANES = 6;
  const DESKTOP_LOAD_MORE_LANES = 24;
  const COMPACT_LOAD_MORE_LANES = 8;

  let prefersReducedMotion = false;
  let compactViewport = true;
  let mounted = false;
  let visiblePrimaryLimit = COMPACT_INITIAL_LANES;
  let previousPrimaryLaneKey = '';
  let previousCompactViewport = compactViewport;
  let lanesElement: HTMLOListElement | null = null;
  let autoLoadSentinelElement: HTMLLIElement | null = null;
  let autoLoadObserver: IntersectionObserver | undefined;
  let stopReducedMotionWatch: (() => void) | undefined;
  let stopCompactViewportWatch: (() => void) | undefined;

  onMount(() => {
    mounted = true;
    stopReducedMotionWatch = watchReducedMotion();
    stopCompactViewportWatch = watchCompactViewport();
    return () => {
      mounted = false;
      disconnectAutoLoadObserver();
      stopReducedMotionWatch?.();
      stopCompactViewportWatch?.();
    };
  });

  $: entities = buildDistributionEntities(rows);
  $: buckets = buildDistributionBuckets(rows, entities, bucketKeys);
  $: activeBuckets = buckets.filter((bucket) => bucket.total > 0);
  $: totalListens = entities.reduce((sum, entity) => sum + entity.total, 0);
  $: lanes = buildDistributionLanes(entities, activeBuckets, totalListens, formatBucketLabel, distributionColor);
  $: primaryLanes = lanes.filter((lane) => !lane.entity.isOther);
  $: otherLane = lanes.find((lane) => lane.entity.isOther);
  $: primaryLaneKey = primaryLanes.map((lane) => lane.entity.id).join(',');
  $: if (primaryLaneKey !== previousPrimaryLaneKey) {
    previousPrimaryLaneKey = primaryLaneKey;
    visiblePrimaryLimit = initialVisibleLaneLimit();
  }
  $: if (compactViewport !== previousCompactViewport) {
    previousCompactViewport = compactViewport;
    visiblePrimaryLimit = initialVisibleLaneLimit();
  }
  $: visiblePrimaryLanes = primaryLanes.slice(0, visiblePrimaryLimit);
  $: remainingPrimaryLanes = Math.max(0, primaryLanes.length - visiblePrimaryLanes.length);
  $: renderedLanes = otherLane ? [...visiblePrimaryLanes, otherLane] : visiblePrimaryLanes;
  $: loadMoreLaneCount = compactViewport ? COMPACT_LOAD_MORE_LANES : DESKTOP_LOAD_MORE_LANES;
  $: autoLoadSignature = [
    mounted,
    compactViewport,
    remainingPrimaryLanes,
    visiblePrimaryLimit,
    Boolean(lanesElement),
    Boolean(autoLoadSentinelElement),
  ].join(':');
  $: if (autoLoadSignature) void syncAutoLoadObserver();
  $: firstBucketLabel = activeBuckets[0] ? formatBucketLabel(activeBuckets[0].bucket) : '';
  $: lastBucket = activeBuckets.at(-1);
  $: lastBucketLabel = lastBucket ? formatBucketLabel(lastBucket.bucket) : '';

  function watchReducedMotion(): () => void {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => {
      prefersReducedMotion = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }

  function watchCompactViewport(): () => void {
    const query = window.matchMedia('(max-width: 860px)');
    const update = () => {
      compactViewport = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }

  function motionDuration(duration: number): number {
    return prefersReducedMotion ? 1 : duration;
  }

  function distributionColor(index: number, entity: DistributionEntity): string {
    return entity.isOther ? 'var(--color-muted)' : chartColor(index);
  }

  function formatDistributionValue(value: number): string {
    return `${formatCountValue(value)} listens`;
  }

  function formatDistributionCell(point: LanePoint): string {
    return `${formatDistributionValue(point.value)}, ${formatPercentValue(point.bucketShare)}`;
  }

  function formatPeak(lane: DistributionLane): string {
    if (!lane.peak) return 'No active bucket';
    return `${formatDistributionValue(lane.peak.value)} ${formatBucketPreposition(lane.peak.bucket)} ${lane.peak.label}`;
  }

  function artistHref(id: string): string {
    return `${pagePrefix}/artist/${id}`;
  }

  function artistTransition(id: string, scope: string): string {
    return viewTransitionName(id, scope);
  }

  function artistTransitionHref(id: string, scope: string): string {
    return transitionHref(artistHref(id), artistTransition(id, scope));
  }

  function transitionScope(index: number): string {
    return `stats-distribution-${index}`;
  }

  function initialVisibleLaneLimit(): number {
    return compactViewport ? COMPACT_INITIAL_LANES : DESKTOP_INITIAL_LANES;
  }

  function showMoreLanes() {
    visiblePrimaryLimit = Math.min(primaryLanes.length, visiblePrimaryLimit + loadMoreLaneCount);
  }

  async function syncAutoLoadObserver() {
    if (!mounted) return;
    await tick();

    disconnectAutoLoadObserver();
    if (
      compactViewport ||
      remainingPrimaryLanes <= 0 ||
      !lanesElement ||
      !autoLoadSentinelElement ||
      !('IntersectionObserver' in window)
    ) return;

    autoLoadObserver = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      showMoreLanes();
    }, {
      root: lanesElement,
      rootMargin: '160px 0px',
      threshold: 0.01,
    });
    autoLoadObserver.observe(autoLoadSentinelElement);
  }

  function disconnectAutoLoadObserver() {
    autoLoadObserver?.disconnect();
    autoLoadObserver = undefined;
  }
</script>

<Card.Root class={`stats-card artist-distribution-card ${className}`}>
  <Card.Header>
    <Card.Description>top artist lanes by {timelineDescription}</Card.Description>
    <Card.Title>Artist listening distribution</Card.Title>
  </Card.Header>
  <Card.Content class="artist-distribution-content">
    {#if activeBuckets.length === 0}
      <p class="state">Not enough artist distribution data for this range.</p>
    {:else}
      <div class="distribution-overview" aria-label="Distribution summary">
        <span>{formatDistributionValue(totalListens)}</span>
        <span>{primaryLanes.length} top artists</span>
        {#if remainingPrimaryLanes > 0}
          <span>showing {visiblePrimaryLanes.length} of {primaryLanes.length}</span>
        {/if}
        {#if otherLane}
          <span>{formatPercentValue(otherLane.share)} other artists</span>
        {/if}
      </div>

      <ol class="artist-lanes" bind:this={lanesElement}>
        {#each visiblePrimaryLanes as lane, index (lane.entity.id)}
          <li in:fade={{ duration: motionDuration(120) }} out:fade={{ duration: motionDuration(80) }}>
            <a
              class="artist-lane"
              href={artistTransitionHref(lane.entity.id, transitionScope(index))}
              style={`--swatch: ${lane.color};`}
              aria-label={`${lane.entity.name}, ${formatDistributionValue(lane.entity.total)}, ${formatPercentValue(lane.share)} of listens`}
            >
              <span class="lane-rank">{String(index + 1).padStart(2, '0')}</span>
              <span class="lane-cover">
                <CoverArt
                  src={directImageUrl(lane.entity)}
                  name={lane.entity.name}
                  size="xs"
                  transitionName={artistTransition(lane.entity.id, transitionScope(index))}
                />
              </span>
              {@render LaneContent(lane)}
            </a>
          </li>
        {/each}

        {#if remainingPrimaryLanes > 0 && !compactViewport}
          <li class="auto-load-sentinel" bind:this={autoLoadSentinelElement} aria-hidden="true"></li>
        {/if}

        {#if otherLane}
          <li class="other-lane" in:fade={{ duration: motionDuration(120) }} out:fade={{ duration: motionDuration(80) }}>
            <div
              class="artist-lane"
              style={`--swatch: ${otherLane.color};`}
              aria-label={`Other artists, ${formatDistributionValue(otherLane.entity.total)}, ${formatPercentValue(otherLane.share)} of listens`}
            >
              <span class="lane-rank" aria-hidden="true"></span>
              <span class="other-mark" aria-hidden="true"></span>
              {@render LaneContent(otherLane)}
            </div>
          </li>
        {/if}
      </ol>

      {#if remainingPrimaryLanes > 0 && compactViewport}
        <div class="lanes-more">
          <Button variant="outline" size="xs" onclick={showMoreLanes}>
            Show {Math.min(loadMoreLaneCount, remainingPrimaryLanes)} more artists
          </Button>
        </div>
      {/if}

      <div class="lane-axis" aria-hidden="true">
        <span>{firstBucketLabel}</span>
        <span>{timelineDescription}</span>
        <span>{lastBucketLabel}</span>
      </div>

      <table class="sr-only">
        <caption>Artist listening distribution data</caption>
        <thead>
          <tr>
            <th scope="col">Artist</th>
            <th scope="col">Time bucket</th>
            <th scope="col">Listens</th>
          </tr>
        </thead>
        <tbody>
          {#each renderedLanes as lane (lane.entity.id)}
            {#each lane.points.filter((point) => point.value > 0) as point (point.bucket)}
              <tr>
                <td>{lane.entity.name}</td>
                <td>{point.label}</td>
                <td>{formatDistributionCell(point)}</td>
              </tr>
            {/each}
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

{#snippet LaneContent(lane: DistributionLane)}
  <span class="lane-copy">
    <strong>{lane.entity.name}</strong>
    <small>{formatDistributionValue(lane.entity.total)}</small>
  </span>
  <span
    class="lane-sparkline"
    style={`grid-template-columns: repeat(${lane.points.length}, minmax(2px, 1fr));`}
    aria-hidden="true"
  >
    {#each lane.points as point (point.bucket)}
      <span class="spark-cell" class:active={point.value > 0} style={`--height: ${point.height}%;`}>
        <span></span>
      </span>
    {/each}
  </span>
  <span class="lane-share">
    <strong>{formatPercentValue(lane.share)}</strong>
    <small>{formatPeak(lane)}</small>
  </span>
{/snippet}

<style>
  :global(.artist-distribution-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.artist-distribution-content) {
    display: grid;
    gap: 0.75rem;
    min-height: 0;
  }

  .distribution-overview {
    display: flex;
    flex-wrap: wrap;
    gap: 0.42rem;
    align-items: center;
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .distribution-overview span {
    border: 1px solid color-mix(in srgb, var(--color-border) 74%, transparent);
    border-radius: var(--radius-xs);
    padding: 0.26rem 0.42rem;
    background: color-mix(in srgb, var(--color-panel) 48%, transparent);
  }

  .artist-lanes {
    display: grid;
    gap: 0.35rem;
    min-width: 0;
    max-height: clamp(18rem, 38vh, 25rem);
    margin: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0 0.25rem 0 0;
    list-style: none;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--color-border) 88%, var(--color-muted)) transparent;
  }

  .artist-lanes::-webkit-scrollbar {
    width: 0.45rem;
  }

  .artist-lanes::-webkit-scrollbar-track {
    background: transparent;
  }

  .artist-lanes::-webkit-scrollbar-thumb {
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 88%, var(--color-muted));
  }

  .artist-lane {
    display: grid;
    grid-template-columns: 1.6rem 2.25rem minmax(7rem, 1fr) minmax(12rem, 2fr) minmax(6.5rem, 0.65fr);
    gap: 0.55rem;
    align-items: center;
    min-width: 0;
    min-height: 3.25rem;
    border: 1px solid color-mix(in srgb, var(--swatch) 18%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.42rem 0.55rem;
    background: color-mix(in srgb, var(--swatch) 5%, transparent);
    color: var(--color-text);
    text-decoration: none;
    transition:
      border-color 150ms var(--ease-out-quart),
      background 150ms var(--ease-out-quart);
  }

  a.artist-lane:hover,
  a.artist-lane:focus-visible {
    border-color: color-mix(in srgb, var(--swatch) 52%, var(--color-border));
    background: color-mix(in srgb, var(--swatch) 9%, var(--color-panel));
    outline: none;
  }

  a.artist-lane:focus-visible {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--swatch) 24%, transparent);
  }

  .lane-rank {
    color: color-mix(in srgb, var(--color-muted) 72%, transparent);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    font-weight: 800;
  }

  .lane-cover {
    display: block;
    width: 2.25rem;
    line-height: 0;
  }

  .lane-copy,
  .lane-share {
    display: grid;
    gap: 0.14rem;
    min-width: 0;
  }

  .lane-copy strong,
  .lane-copy small,
  .lane-share strong,
  .lane-share small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lane-copy strong {
    font-size: 0.86rem;
    line-height: 1.05;
  }

  .lane-copy small,
  .lane-share small {
    color: var(--color-muted);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
  }

  .lane-share {
    justify-items: end;
    text-align: right;
  }

  .lane-share strong {
    color: var(--color-text);
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
  }

  .lane-sparkline {
    display: grid;
    gap: 2px;
    align-items: end;
    min-width: 0;
    height: 2rem;
    padding-block: 0.15rem;
  }

  .spark-cell {
    position: relative;
    display: block;
    height: 100%;
    min-width: 2px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 40%, transparent);
  }

  .spark-cell span {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    display: block;
    height: var(--height, 0%);
    border-radius: inherit;
    background: color-mix(in srgb, var(--swatch) 82%, var(--color-panel));
    opacity: 0;
    transition:
      height 180ms var(--ease-out-quart),
      opacity 180ms var(--ease-out-quart);
  }

  .spark-cell.active span {
    opacity: 0.9;
  }

  .other-lane .artist-lane {
    border-color: color-mix(in srgb, var(--color-border) 62%, transparent);
    background: color-mix(in srgb, var(--color-panel) 30%, transparent);
    color: var(--color-muted);
  }

  .other-lane .spark-cell span {
    background: color-mix(in srgb, var(--color-muted) 38%, var(--color-panel));
    opacity: 0.55;
  }

  .other-mark {
    display: block;
    width: 2.25rem;
    aspect-ratio: 1;
    border: 1px solid color-mix(in srgb, var(--color-border) 74%, transparent);
    border-radius: var(--radius-xs);
    background:
      linear-gradient(135deg, transparent 35%, color-mix(in srgb, var(--color-muted) 28%, transparent) 35% 50%, transparent 50%),
      color-mix(in srgb, var(--color-panel-2) 84%, transparent);
  }

  .lanes-more {
    display: flex;
    justify-content: center;
  }

  .auto-load-sentinel {
    height: 1px;
    margin-top: -0.35rem;
    pointer-events: none;
  }

  .lane-axis {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 0.75rem;
    align-items: center;
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 720;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .lane-axis span:nth-child(2) {
    justify-self: center;
    color: color-mix(in srgb, var(--color-muted) 80%, transparent);
  }

  .lane-axis span:last-child {
    justify-self: end;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 860px) {
    .artist-lanes {
      max-height: none;
      overflow: visible;
      padding-right: 0;
    }

    .artist-lane {
      grid-template-columns: 1.35rem 2.25rem minmax(0, 1fr) minmax(3.4rem, auto);
      gap: 0.48rem;
      align-items: center;
    }

    .lane-rank,
    .lane-cover,
    .other-mark,
    .lane-copy,
    .lane-share {
      grid-row: 1;
    }

    .lane-rank {
      grid-column: 1;
    }

    .lane-cover,
    .other-mark {
      grid-column: 2;
    }

    .lane-copy {
      grid-column: 3;
    }

    .lane-share {
      grid-column: 4;
    }

    .lane-sparkline {
      grid-column: 3 / 5;
      grid-row: 2;
      height: 1.65rem;
      margin-top: 0.1rem;
    }

    .lane-share small {
      display: none;
    }
  }

  @media (max-width: 520px) {
    .distribution-overview {
      gap: 0.32rem;
      font-size: 0.66rem;
    }

    .lane-axis span:nth-child(2) {
      display: none;
    }

    .lane-axis {
      grid-template-columns: 1fr 1fr;
    }

    .artist-lane {
      grid-template-columns: 1.1rem 2rem minmax(0, 1fr) auto;
      min-height: 3rem;
      padding: 0.38rem 0.42rem;
    }

    .lane-cover {
      --cover-size: 2rem;
    }

    .lane-copy small,
    .lane-share small {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .artist-lane,
    .spark-cell span {
      transition: none;
    }
  }

</style>
