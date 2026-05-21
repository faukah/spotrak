<script lang="ts">
  import { initials } from '../../lib/images';

  export let src: string | null | undefined = null;
  export let name = 'Cover';
  export let href: string | undefined = undefined;
  export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'md';
  export let transitionName: string | undefined = undefined;

  $: style = transitionName ? `view-transition-name: ${transitionName}; view-transition-class: cover-art;` : undefined;
</script>

{#if href}
  <a class={`cover ${size}`} {href} aria-label={name} {style}>
    {#if src}
      <img src={src} alt={name} loading="lazy" decoding="async" />
    {:else}
      <span>{initials(name)}</span>
    {/if}
  </a>
{:else}
  <div class={`cover ${size}`} aria-label={name} {style}>
    {#if src}
      <img src={src} alt={name} loading="lazy" decoding="async" />
    {:else}
      <span>{initials(name)}</span>
    {/if}
  </div>
{/if}

<style>
  .cover {
    position: relative;
    display: block;
    flex: 0 0 auto;
    width: var(--cover-size, 4rem);
    aspect-ratio: 1;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--color-border) 86%, transparent);
    border-radius: var(--radius-xs);
    background: var(--color-panel-2);
    box-shadow: 0 1px 0 oklch(0.98 0.01 85 / 0.04) inset, 0 14px 30px oklch(0.08 0.012 255 / 0.28);
    color: var(--color-text);
    contain: paint;
    isolation: isolate;
    text-decoration: none;
    transform: translateZ(0);
    transition:
      border-color var(--motion-feedback) var(--ease-out-quart),
      box-shadow var(--motion-state) var(--ease-out-quart);
  }

  .xs { --cover-size: 2.25rem; }
  .sm { --cover-size: 3rem; }
  .md { --cover-size: 4.25rem; }
  .lg { --cover-size: clamp(7rem, 16vw, 12rem); }
  .xl { --cover-size: clamp(11rem, 28vw, 22rem); }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition:
      transform var(--motion-cover-hover) var(--ease-out-quart),
      filter var(--motion-cover-hover) var(--ease-out-quart);
  }

  span {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: color-mix(in srgb, var(--color-text) 80%, transparent);
    font-size: clamp(1rem, 4vw, 3rem);
    font-weight: 800;
    letter-spacing: -0.08em;
  }

  a.cover:hover,
  a.cover:focus-visible {
    border-color: color-mix(in srgb, var(--color-primary) 48%, var(--color-border));
  }

  a.cover:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--color-primary) 28%, transparent),
      0 14px 30px oklch(0.08 0.012 255 / 0.28);
  }

  a.cover:hover img {
    transform: scale(1.035);
    filter: saturate(1.04) contrast(1.02);
  }

  @media (prefers-reduced-motion: reduce) {
    img {
      transition: none;
    }

    a.cover:hover img {
      transform: none;
      filter: none;
    }
  }
</style>
