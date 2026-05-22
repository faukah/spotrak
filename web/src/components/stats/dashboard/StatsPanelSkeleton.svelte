<script lang="ts">
  type SkeletonKind = "chart" | "split" | "heatmap" | "wide" | "compact";

  export let kind: SkeletonKind = "chart";
  export let className = "";
  export let index = 0;

  const chartBars = [56, 74, 42, 86, 61, 70, 38, 92, 64, 52, 78, 45];
  const compactBars = [62, 44, 70, 52, 80, 58, 66];
  const heatmapCells = Array.from({ length: 56 }, (_, cell) => cell);
</script>

<div
  class={`stats-panel-skeleton ${className} ${kind}`}
  style={`--i: ${index};`}
  aria-hidden="true"
>
  <div class="skeleton-header">
    <span class="skeleton skeleton-line eyebrow"></span>
    <strong class="skeleton skeleton-line title"></strong>
  </div>

  {#if kind === "heatmap"}
    <div class="skeleton-heatmap">
      {#each heatmapCells as cell (cell)}
        <span class="skeleton" style={`--tone: ${(cell % 7) + 1};`}></span>
      {/each}
    </div>
  {:else if kind === "split"}
    <div class="skeleton-split">
      {#each [0, 1] as group (group)}
        <div class="skeleton-chart-frame small">
          {#each compactBars as height, bar (bar)}
            <span
              class="skeleton bar"
              style={`--height: ${Math.max(26, height - group * 11)}%;`}
            ></span>
          {/each}
        </div>
      {/each}
    </div>
  {:else}
    <div class={`skeleton-chart-frame ${kind === "wide" ? "wide" : ""} ${kind === "compact" ? "compact" : ""}`}>
      {#each (kind === "compact" ? compactBars : chartBars) as height, bar (bar)}
        <span class="skeleton bar" style={`--height: ${height}%;`}></span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .stats-panel-skeleton {
    display: grid;
    gap: 0.9rem;
    min-height: 22rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 86%, transparent);
    box-shadow: var(--shadow-card);
  }

  .stats-panel-skeleton.compact {
    min-height: 18rem;
  }

  .skeleton-header {
    display: grid;
    gap: 0.42rem;
    align-content: start;
  }

  .skeleton-line {
    display: block;
    height: 0.75rem;
    border-radius: 999px;
  }

  .skeleton-line.eyebrow {
    width: min(7.2rem, 42%);
    height: 0.52rem;
  }

  .skeleton-line.title {
    width: min(13rem, 68%);
    height: 1rem;
  }

  .skeleton-chart-frame {
    position: relative;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    align-items: end;
    gap: 0.5rem;
    min-height: 16.8rem;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    border-radius: var(--radius-md);
    padding: 1rem 0.9rem 0.85rem;
    background:
      linear-gradient(
        to bottom,
        transparent 0,
        transparent calc(25% - 1px),
        color-mix(in srgb, var(--color-border) 54%, transparent) 25%,
        transparent calc(25% + 1px),
        transparent calc(50% - 1px),
        color-mix(in srgb, var(--color-border) 46%, transparent) 50%,
        transparent calc(50% + 1px),
        transparent calc(75% - 1px),
        color-mix(in srgb, var(--color-border) 38%, transparent) 75%,
        transparent calc(75% + 1px)
      ),
      color-mix(in srgb, var(--color-panel-2) 42%, transparent);
  }

  .skeleton-chart-frame.wide {
    min-height: 17.2rem;
  }

  .skeleton-chart-frame.compact {
    grid-template-columns: repeat(7, minmax(0, 1fr));
    min-height: 12.6rem;
  }

  .skeleton-chart-frame.small {
    grid-template-columns: repeat(7, minmax(0, 1fr));
    min-height: 7.5rem;
  }

  .bar {
    display: block;
    height: var(--height);
    min-height: 1rem;
    border-radius: 999px 999px 0 0;
  }

  .skeleton-split {
    display: grid;
    gap: 0.75rem;
  }

  .skeleton-heatmap {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 0.38rem;
    align-content: start;
    min-height: 14rem;
    border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    border-radius: var(--radius-md);
    padding: 0.85rem;
    background: color-mix(in srgb, var(--color-panel-2) 38%, transparent);
  }

  .skeleton-heatmap span {
    aspect-ratio: 1;
    min-height: 0.85rem;
    border-radius: 0.28rem;
    opacity: calc(0.32 + var(--tone) * 0.055);
  }

  @media (max-width: 680px) {
    .stats-panel-skeleton,
    .stats-panel-skeleton.compact {
      min-height: 16rem;
    }

    .skeleton-chart-frame,
    .skeleton-chart-frame.wide {
      min-height: 12.5rem;
    }
  }
</style>
