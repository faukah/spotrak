<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Search } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { EntityRef, SearchResults } from '../../lib/api/types';

  export let id = 'library-search';
  export let name = id;

  let query = '';
  let results: SearchResults | null = null;
  let open = false;
  let loading = false;
  let timer: number | undefined;
  let controller: AbortController | null = null;
  let requestId = 0;

  onDestroy(() => {
    if (timer) window.clearTimeout(timer);
    controller?.abort();
  });

  $: groups = results
    ? [
        { label: 'Tracks', href: 'track', items: results.tracks },
        { label: 'Artists', href: 'artist', items: results.artists },
        { label: 'Albums', href: 'album', items: results.albums },
      ].filter((group) => group.items.length > 0)
    : [];

  function scheduleSearch() {
    if (timer) window.clearTimeout(timer);
    const value = query.trim();
    if (value.length < 2) {
      requestId += 1;
      controller?.abort();
      results = null;
      open = false;
      return;
    }
    timer = window.setTimeout(() => void search(value), 180);
  }

  async function search(value: string) {
    const id = ++requestId;
    controller?.abort();
    controller = new AbortController();
    loading = true;
    open = true;
    try {
      const nextResults = await apiFetch<SearchResults>(`/search?q=${encodeURIComponent(value)}`, { signal: controller.signal });
      if (id === requestId) results = nextResults;
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      if (id === requestId) results = null;
    } finally {
      if (id === requestId) loading = false;
    }
  }

  function resultHref(group: { href: string }, item: EntityRef) {
    return `/${group.href}/${item.id}`;
  }
</script>

<div class="search-shell" onfocusout={() => window.setTimeout(() => (open = false), 120)}>
  <Search class="search-icon" aria-hidden="true" />
  <input
    id={id}
    name={name}
    bind:value={query}
    oninput={scheduleSearch}
    onfocus={() => (open = !!results || query.trim().length >= 2)}
    placeholder="Search library"
    aria-label="Search tracks, artists, albums"
  />
  {#if open}
    <div class="results">
      {#if loading}
        <p>Searching…</p>
      {:else if groups.length === 0}
        <p>No matches.</p>
      {:else}
        {#each groups as group}
          <section>
            <strong>{group.label}</strong>
            {#each group.items as item}
              <a href={resultHref(group, item)}>{item.name}</a>
            {/each}
          </section>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .search-shell {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(8rem, 16rem);
    gap: 0.4rem;
    align-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 0.55rem;
    background: color-mix(in srgb, var(--color-panel) 78%, transparent);
  }

  :global(.search-icon) {
    width: 0.9rem;
    color: var(--color-muted);
  }

  input {
    min-height: 2rem;
    border: 0;
    background: transparent;
    padding: 0;
    outline: none;
  }

  .results {
    position: absolute;
    top: calc(100% + 0.35rem);
    right: 0;
    z-index: 50;
    display: grid;
    gap: 0.6rem;
    width: min(24rem, 80vw);
    max-height: 28rem;
    overflow: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.65rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 96%, black);
    box-shadow: var(--shadow-card);
  }

  section {
    display: grid;
    gap: 0.15rem;
  }

  strong {
    color: var(--color-muted);
    font-size: 0.7rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  a,
  p {
    margin: 0;
    overflow: hidden;
    padding: 0.35rem 0.4rem;
    border-radius: var(--radius-xs);
    color: var(--color-text);
    font-size: 0.86rem;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  a:hover {
    background: var(--color-panel-2);
  }

  p {
    color: var(--color-muted);
  }

  @media (max-width: 900px) {
    .search-shell {
      grid-template-columns: auto minmax(8rem, 1fr);
      width: 100%;
    }
  }

  @media (max-width: 520px) {
    .results {
      left: 0;
      right: auto;
      width: calc(100vw - (2 * var(--space-page)));
      max-height: min(28rem, 68vh);
    }
  }
</style>
