<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, ChevronDown } from '@lucide/svelte';
  import { Button, type ButtonSize } from '../ui/button';
  import type { StatsRangeKey } from '../../lib/api/types';
  import {
    selectedStatsRange,
    statsRangeLabel,
    statsRangeOptions,
  } from '../../lib/stores/stats-range';

  export let availableYears: number[] = [];
  export let ariaLabel = 'Choose time range';
  export let buttonSize: ButtonSize = 'sm';
  export let align: 'start' | 'end' = 'end';

  const currentYear = new Date().getFullYear();

  let menuElement: HTMLDivElement | null = null;
  let open = false;

  $: years = availableYears.length > 0 ? uniqueYears(availableYears) : [currentYear];
  $: selectedRangeText = statsRangeLabel($selectedStatsRange);

  onMount(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (!open || !menuElement || !(event.target instanceof Node)) return;
      if (!menuElement.contains(event.target)) open = false;
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') open = false;
    };

    document.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  });

  function setRange(range: StatsRangeKey) {
    selectedStatsRange.set(range === 'selected-year' ? { range, year: years[0] ?? currentYear } : { range });
    open = false;
  }

  function chooseYear(year: number) {
    selectedStatsRange.set({ range: 'selected-year', year });
    open = false;
  }

  function uniqueYears(values: number[]): number[] {
    return Array.from(new Set(values.filter((year) => Number.isInteger(year))))
      .toSorted((a, b) => b - a);
  }
</script>

<div class="range-menu" data-align={align} bind:this={menuElement}>
  <Button
    variant="outline"
    size={buttonSize}
    class="range-trigger"
    aria-haspopup="true"
    aria-expanded={open}
    aria-label={`${selectedRangeText}. ${ariaLabel}`}
    onclick={() => (open = !open)}
  >
    <span>{selectedRangeText}</span>
    <ChevronDown class="range-trigger-icon" aria-hidden="true" />
  </Button>
  {#if open}
    <div class="range-dropdown" aria-label={ariaLabel}>
      <div class="dropdown-group">
        {#each statsRangeOptions as option (option.key)}
          <button type="button" aria-pressed={$selectedStatsRange.range === option.key} class:active-range={$selectedStatsRange.range === option.key} onclick={() => setRange(option.key)}>
            <span>{option.label}</span>
            {#if $selectedStatsRange.range === option.key}<Check aria-hidden="true" />{/if}
          </button>
        {/each}
      </div>
      <div class="dropdown-separator"></div>
      <span class="dropdown-label">Years</span>
      <div class="dropdown-group years">
        {#each years as year (year)}
          <button type="button" aria-pressed={$selectedStatsRange.range === 'selected-year' && $selectedStatsRange.year === year} class:active-range={$selectedStatsRange.range === 'selected-year' && $selectedStatsRange.year === year} onclick={() => chooseYear(year)}>
            <span>{year}</span>
            {#if $selectedStatsRange.range === 'selected-year' && $selectedStatsRange.year === year}<Check aria-hidden="true" />{/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .range-menu {
    position: relative;
    width: max-content;
  }

  :global(.range-trigger) {
    min-width: 7.5rem;
    justify-content: space-between;
  }

  :global(.range-trigger-icon) {
    width: 0.9rem;
    height: 0.9rem;
  }

  .range-dropdown {
    position: absolute;
    z-index: 40;
    top: calc(100% + 0.35rem);
    right: 0;
    display: grid;
    width: min(18rem, calc(100vw - var(--space-page) - var(--space-page)));
    gap: 0.35rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.45rem;
    background: var(--color-panel);
    box-shadow: var(--shadow-card);
  }

  .range-menu[data-align='start'] .range-dropdown {
    right: auto;
    left: 0;
  }

  .dropdown-group {
    display: grid;
    gap: 0.15rem;
  }

  .dropdown-group.years {
    max-height: 13rem;
    overflow-y: auto;
  }

  .dropdown-separator {
    height: 1px;
    background: var(--color-border);
  }

  .dropdown-label {
    padding: 0.2rem 0.35rem 0;
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .range-dropdown button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.7rem;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 0.48rem 0.5rem;
    background: transparent;
    color: var(--color-text);
    font-size: 0.84rem;
    font-weight: 650;
    text-align: left;
    cursor: pointer;
  }

  .range-dropdown button:hover,
  .range-dropdown button.active-range {
    background: var(--color-panel-2);
    color: var(--color-primary);
  }

  .range-dropdown button :global(svg) {
    width: 0.9rem;
    height: 0.9rem;
  }

  @media (max-width: 620px) {
    .range-menu,
    :global(.range-trigger) {
      width: 100%;
    }

    .range-dropdown,
    .range-menu[data-align='start'] .range-dropdown {
      right: auto;
      left: 0;
      width: min(18rem, calc(100vw - var(--space-page) - var(--space-page)));
    }
  }
</style>
