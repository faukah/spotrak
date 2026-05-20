<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch, loginUrl } from '../../lib/api/client';
  import { isUnauthorized } from '../../lib/auth/redirect';
  import type { MeResponse } from '../../lib/api/types';
  import { Button } from '../ui/button';

  let loaded = false;
  let me: MeResponse | null = null;

  onMount(async () => {
    try {
      me = await apiFetch<MeResponse>('/auth/me');
    } catch (error) {
      if (!isUnauthorized(error)) me = null;
    } finally {
      loaded = true;
    }
  });
</script>

{#if loaded && !me}
  <Button href={loginUrl()} variant="default" size="lg">Connect Spotify</Button>
{:else if loaded && me}
  <p class="signed-in">{me.user.username} · private library</p>
{/if}

<style>
  .signed-in {
    margin: 0;
    color: var(--color-muted);
    font-size: 0.9rem;
  }
</style>
