<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch, ApiError } from '../../lib/api/client';
  import type { SummaryStats } from '../../lib/api/types';

  export let token: string;

  let revoked = false;
  let interval: number | undefined;

  onMount(() => {
    void check();
    interval = window.setInterval(() => void check(), 10_000);
    return () => {
      if (interval) window.clearInterval(interval);
    };
  });

  async function check() {
    if (!token || revoked) return;
    try {
      await apiFetch<SummaryStats>(`/public/${token}/stats/summary`, { cache: 'no-store' });
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        revoked = true;
      }
    }
  }
</script>

{#if revoked}
  <div class="revoked" role="alert">
    <section>
      <p class="kicker">Public link unavailable</p>
      <h1>This share link was revoked.</h1>
      <p>Ask the owner for a new link if you should still have access.</p>
    </section>
  </div>
{/if}

<style>
  .revoked {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: color-mix(in srgb, var(--color-bg) 88%, transparent);
    backdrop-filter: blur(18px);
  }

  section {
    max-width: 42rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: clamp(1rem, 4vw, 2rem);
    background: var(--color-bg-elevated);
    box-shadow: var(--shadow-card);
  }

  h1 {
    margin: 0;
    font-size: clamp(2rem, 5vw, 4rem);
    line-height: 0.92;
    letter-spacing: -0.075em;
  }

  p:not(.kicker) {
    margin: 0.75rem 0 0;
    color: var(--color-muted);
  }
</style>
