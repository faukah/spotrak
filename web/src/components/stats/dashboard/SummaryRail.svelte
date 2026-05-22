<script lang="ts">
  import { fade } from "svelte/transition";

  type SummaryItem = {
    label: string;
    value: string;
    detail: string;
  };

  export let items: SummaryItem[] = [];
  export let loading = false;
  export let motionDuration: (duration: number) => number = (duration) => duration;

  const loadingItems: SummaryItem[] = [
    { label: "Listens", value: "", detail: "" },
    { label: "Time listened", value: "", detail: "" },
    { label: "Different artists", value: "", detail: "" },
    { label: "Average release year", value: "", detail: "" },
    { label: "Average features per track", value: "", detail: "" },
  ];
</script>

<section class="stat-rail stats-section-reveal" aria-label="Selected range summary" aria-busy={loading}>
  {#if loading}
    {#each loadingItems as item, index (item.label)}
      <article class="stat-rail-skeleton" style={`--i: ${index};`}>
        <span>{item.label}</span>
        <strong class="skeleton skeleton-line value"></strong>
        <small class="skeleton skeleton-line"></small>
      </article>
    {/each}
  {:else}
    {#each items as item, index (item.label)}
      <article style={`--i: ${index};`} in:fade={{ duration: motionDuration(120) }}>
        <span>{item.label}</span>
        <strong>{item.value}</strong>
        <small>{item.detail}</small>
      </article>
    {/each}
  {/if}
</section>

<style>
  .stat-rail {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-bg-elevated) 82%, transparent);
  }

  .stat-rail article {
    display: grid;
    gap: 0.14rem;
    min-width: 0;
    border-right: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    padding: 0.62rem 0.75rem;
  }

  .stat-rail article:last-child {
    border-right: 0;
  }

  .stat-rail span,
  .stat-rail small {
    color: var(--color-muted);
    font-size: 0.72rem;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .stat-rail small {
    overflow: hidden;
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-rail strong {
    font-size: clamp(1.02rem, 1.45vw, 1.34rem);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
    line-height: 1;
  }

  .stat-rail-skeleton .skeleton-line {
    width: min(8rem, 78%);
    height: 0.64rem;
    border-radius: 999px;
  }

  .stat-rail-skeleton .skeleton-line.value {
    width: min(7.4rem, 72%);
    height: 1.25rem;
  }

  @media (max-width: 1180px) {
    .stat-rail {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .stat-rail article {
      border-right: 0;
      border-bottom: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    }

    .stat-rail article:nth-child(odd) {
      border-right: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
    }

    .stat-rail article:last-child,
    .stat-rail article:nth-last-child(2):nth-child(odd) {
      border-bottom: 0;
    }
  }

  @media (max-width: 680px) {
    .stat-rail {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .stat-rail article {
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: baseline;
    }

    .stat-rail article:last-child {
      grid-column: 1 / -1;
    }

    .stat-rail small {
      grid-column: 1 / -1;
    }
  }
</style>
