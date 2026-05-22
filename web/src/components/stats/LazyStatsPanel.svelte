<script lang="ts" generics="Props extends Record<string, unknown>">
  import { onMount, type Component } from "svelte";
  import StatsPanelSkeleton from "./dashboard/StatsPanelSkeleton.svelte";

  type SkeletonKind = "chart" | "split" | "heatmap" | "wide" | "compact";
  type LazyModule = { default: Component<Props> };

  export let load: () => Promise<LazyModule> = () =>
    Promise.reject(new Error("No panel loader configured"));
  export let props = {} as Props;
  export let className = "";
  export let skeletonKind: SkeletonKind = "chart";
  export let index = 0;
  export let rootMargin = "420px 0px";
  export let loading = false;

  let host: HTMLDivElement | null = null;
  let component: Component<Props> | null = null;
  let componentLoading = false;
  let loadError: string | null = null;
  let observer: IntersectionObserver | undefined;
  let visible = false;

  $: if (visible && !loading) void loadComponent();

  onMount(() => {
    if (!host || typeof IntersectionObserver === "undefined") {
      visible = true;
      return;
    }

    observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          observer?.disconnect();
          visible = true;
        }
      },
      { rootMargin },
    );
    observer.observe(host);

    return () => observer?.disconnect();
  });

  async function loadComponent() {
    if (component || componentLoading) return;
    componentLoading = true;
    loadError = null;
    try {
      component = (await load()).default;
    } catch (error) {
      loadError = error instanceof Error ? error.message : "Unable to load panel";
    } finally {
      componentLoading = false;
    }
  }
</script>

<div bind:this={host} class={`lazy-stats-panel ${className}`} style={`--i: ${index};`}>
  {#if loading}
    <StatsPanelSkeleton kind={skeletonKind} {index} />
  {:else if component}
    <svelte:component this={component} {...props} />
  {:else if loadError}
    <div class="lazy-panel-error" role="alert">{loadError}</div>
  {:else}
    <StatsPanelSkeleton kind={skeletonKind} {index} />
  {/if}
</div>

<style>
  .lazy-stats-panel {
    min-width: 0;
    height: 100%;
  }

  .lazy-stats-panel :global(.stats-panel-skeleton) {
    height: 100%;
  }

  .lazy-panel-error {
    min-height: 12rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1rem;
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-bg-elevated) 86%, transparent);
    box-shadow: var(--shadow-card);
  }
</style>
