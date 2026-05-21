<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Search } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { EntityRef, SearchResults } from '../../lib/api/types';

  export let id = 'library-search';
  export let name = id;

  type ResultGroup = { label: string; href: string; items: EntityRef[] };
  type ResultEntry = { group: ResultGroup; item: EntityRef; href: string; optionId: string };

  let query = '';
  let results: SearchResults | null = null;
  let open = false;
  let loading = false;
  let timer: number | undefined;
  let controller: AbortController | null = null;
  let requestId = 0;
  let activeIndex = -1;
  let shellElement: HTMLDivElement | null = null;
  let resultsElement: HTMLDivElement | null = null;
  let pendingScrollId: string | undefined;

  $: resultsId = `${id}-results`;
  $: groups = results
    ? [
        { label: 'Tracks', href: 'track', items: results.tracks },
        { label: 'Artists', href: 'artist', items: results.artists },
        { label: 'Albums', href: 'album', items: results.albums },
      ].filter((group) => group.items.length > 0)
    : [];
  $: optionEntries = resultEntries(groups);
  $: activeId = open && activeIndex >= 0 ? optionEntries[activeIndex]?.optionId : undefined;
  $: if (activeId) void scrollActiveOptionIntoView(activeId);
  $: if (!open || optionEntries.length === 0) {
    activeIndex = -1;
  } else if (activeIndex >= optionEntries.length) {
    activeIndex = optionEntries.length - 1;
  }

  onDestroy(() => {
    if (timer) window.clearTimeout(timer);
    controller?.abort();
  });

  function scheduleSearch() {
    if (timer) window.clearTimeout(timer);
    const value = query.trim();
    activeIndex = -1;
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
    const searchRequestId = ++requestId;
    controller?.abort();
    controller = new AbortController();
    loading = true;
    open = true;
    try {
      const nextResults = await apiFetch<SearchResults>(`/search?q=${encodeURIComponent(value)}`, { signal: controller.signal });
      if (searchRequestId === requestId) results = nextResults;
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      if (searchRequestId === requestId) results = null;
    } finally {
      if (searchRequestId === requestId) loading = false;
    }
  }

  function resultHref(group: { href: string }, item: EntityRef) {
    return `/${group.href}/${item.id}`;
  }

  function resultEntries(inputGroups: ResultGroup[]): ResultEntry[] {
    return inputGroups.flatMap((group) => group.items.map((item, index) => ({
      group,
      item,
      href: resultHref(group, item),
      optionId: `${id}-${group.href}-${item.id}-${index}`,
    })));
  }

  function openResults() {
    if (results || query.trim().length >= 2) open = true;
  }

  function closeResults() {
    open = false;
    activeIndex = -1;
  }

  function navigateTo(entry: ResultEntry | undefined) {
    if (!entry) return;
    closeResults();
    window.location.assign(entry.href);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (open) event.preventDefault();
      closeResults();
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      openResults();
      if (optionEntries.length > 0) activeIndex = (activeIndex + 1) % optionEntries.length;
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      openResults();
      if (optionEntries.length > 0) activeIndex = activeIndex <= 0 ? optionEntries.length - 1 : activeIndex - 1;
      return;
    }

    if (event.key === 'Enter' && open && activeIndex >= 0) {
      event.preventDefault();
      navigateTo(optionEntries[activeIndex]);
    }
  }

  async function scrollActiveOptionIntoView(optionId: string) {
    pendingScrollId = optionId;
    await tick();
    if (pendingScrollId !== optionId || !resultsElement) return;

    const option = document.getElementById(optionId);
    if (!option || !resultsElement.contains(option)) return;

    const optionScrollMargin = 8;
    const optionRect = option.getBoundingClientRect();
    const resultsRect = resultsElement.getBoundingClientRect();
    const visibleTop = resultsRect.top + optionScrollMargin;
    const visibleBottom = resultsRect.bottom - optionScrollMargin;
    if (optionRect.top < visibleTop) {
      resultsElement.scrollTop -= visibleTop - optionRect.top;
    } else if (optionRect.bottom > visibleBottom) {
      resultsElement.scrollTop += optionRect.bottom - visibleBottom;
    }
  }

  function handleFocusOut() {
    window.setTimeout(() => {
      if (!shellElement?.contains(document.activeElement)) closeResults();
    }, 0);
  }
</script>

<div class="search-shell" bind:this={shellElement} onfocusout={handleFocusOut}>
  <Search class="search-icon" aria-hidden="true" />
  <input
    role="combobox"
    id={id}
    name={name}
    bind:value={query}
    oninput={scheduleSearch}
    onfocus={openResults}
    onkeydown={handleKeyDown}
    placeholder="Search library"
    aria-label="Search tracks, artists, albums"
    aria-autocomplete="list"
    aria-expanded={open}
    aria-controls={resultsId}
    aria-activedescendant={activeId}
  />
  {#if open}
    <div class="results" id={resultsId} role="listbox" aria-label="Search results" bind:this={resultsElement}>
      {#if loading}
        <p role="status" aria-live="polite">Searching…</p>
      {:else if groups.length === 0}
        <p role="status">No matches.</p>
      {:else}
        {#each groups as group}
          <section role="group" aria-label={group.label}>
            <strong aria-hidden="true">{group.label}</strong>
            {#each group.items as item}
              {@const entryIndex = optionEntries.findIndex((entry) => entry.group === group && entry.item.id === item.id)}
              {@const entry = optionEntries[entryIndex]}
              <a
                id={entry.optionId}
                role="option"
                aria-selected={activeIndex === entryIndex}
                href={entry.href}
                onmouseenter={() => (activeIndex = entryIndex)}
                onfocus={() => (activeIndex = entryIndex)}
                onclick={closeResults}
              >{item.name}</a>
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
    transition: border-color 140ms ease, background 140ms ease;
  }

  .search-shell:focus-within {
    border-color: color-mix(in srgb, var(--color-primary) 62%, var(--color-border));
    background: var(--color-panel);
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
    background: color-mix(in srgb, var(--color-bg-elevated) 96%, var(--color-bg));
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

  a:hover,
  a[aria-selected="true"] {
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

  @media (pointer: coarse) {
    input {
      min-height: 2.75rem;
      font-size: 1rem;
    }

    a,
    p {
      min-height: 2.75rem;
      padding-block: 0.7rem;
    }
  }
</style>
