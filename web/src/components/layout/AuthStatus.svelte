<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch, loginUrl } from '../../lib/api/client';
  import { isUnauthorized } from '../../lib/auth/redirect';
  import type { MeResponse } from '../../lib/api/types';
  import { Button } from '../ui/button';

  let me: MeResponse | null = null;
  let loading = true;

  onMount(() => {
    void load();
    const onUnauthorized = () => {
      me = null;
    };
    window.addEventListener('spotrak:unauthorized', onUnauthorized);
    return () => window.removeEventListener('spotrak:unauthorized', onUnauthorized);
  });

  async function load() {
    try {
      me = await apiFetch<MeResponse>('/auth/me');
    } catch (error) {
      if (!isUnauthorized(error)) me = null;
    } finally {
      loading = false;
    }
  }

  async function logout() {
    await apiFetch<void>('/auth/logout', { method: 'POST' });
    window.location.href = '/login';
  }
</script>

{#if loading}
  <span class="auth muted">Session…</span>
{:else if me}
  <div class="auth signed-in">
    <span title={me.user.username}>{me.user.username}</span>
    <Button variant="outline" size="sm" onclick={logout}>Log out</Button>
  </div>
{:else}
  <Button href={loginUrl()} variant="outline" size="sm">Sign in</Button>
{/if}

<style>
  .auth {
    white-space: nowrap;
  }

  .signed-in {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--color-muted);
    font-size: 0.86rem;
  }

  .signed-in span {
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @media (max-width: 520px) {
    .signed-in span {
      display: none;
    }
  }
</style>
