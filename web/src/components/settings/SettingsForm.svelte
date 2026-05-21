<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { MeResponse } from '../../lib/api/types';
  import { applyThemePreference, setThemePreference, type ThemePreference } from '../../lib/theme';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';
  import { Input } from '../ui/input';

  let me: MeResponse | null = null;
  let timezone = '';
  let metric = 'number';
  let hourFormat: '12' | '24' = '24';
  let darkMode: ThemePreference = 'follow';
  let loading = true;
  let saving = false;
  let error: string | null = null;
  let settingsSaved = false;
  let linkCopied = false;
  let publicSharingEnabled = false;
  let publicToken: string | null = null;
  let sharingBusy = false;
  let spotifyBusy = false;
  let spotifyDisconnected = false;
  let confirmDisconnect = false;

  $: publicLink = publicToken && typeof window !== 'undefined' ? `${window.location.origin}/public/${publicToken}` : '';

  onMount(() => {
    const handleThemeChange = (event: Event) => {
      darkMode = (event as CustomEvent<{ preference: ThemePreference }>).detail.preference;
    };
    window.addEventListener('spotrak:theme-change', handleThemeChange);
    void loadSettings();
    return () => window.removeEventListener('spotrak:theme-change', handleThemeChange);
  });

  async function loadSettings() {
    try {
      me = await apiFetch<MeResponse>('/users/me');
      timezone = me.settings.timezone ?? '';
      metric = me.settings.metric_used;
      hourFormat = me.settings.hour_format ?? '24';
      darkMode = me.settings.dark_mode;
      applyThemePreference(darkMode);
      publicSharingEnabled = me.public_sharing.enabled;
      publicToken = me.public_sharing.token ?? null;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load settings';
    } finally {
      loading = false;
    }
  }

  async function enableOrRotateSharing() {
    sharingBusy = true;
    error = null;
    try {
      const response = await apiFetch<{ token: string }>('/users/me/public-token', { method: 'POST' });
      publicSharingEnabled = true;
      publicToken = response.token;
      linkCopied = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to update public sharing';
    } finally {
      sharingBusy = false;
    }
  }

  async function revokeSharing() {
    sharingBusy = true;
    error = null;
    try {
      await apiFetch<void>('/users/me/public-token', { method: 'DELETE' });
      publicSharingEnabled = false;
      publicToken = null;
      linkCopied = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to revoke public sharing';
    } finally {
      sharingBusy = false;
    }
  }

  async function copyPublicLink() {
    if (!publicLink) return;
    await navigator.clipboard.writeText(publicLink);
    linkCopied = true;
  }

  async function disconnectSpotify() {
    spotifyBusy = true;
    spotifyDisconnected = false;
    error = null;
    try {
      await apiFetch<void>('/users/me/spotify-connection', { method: 'DELETE' });
      spotifyDisconnected = true;
      confirmDisconnect = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to disconnect Spotify';
    } finally {
      spotifyBusy = false;
    }
  }

  async function submit() {
    saving = true;
    settingsSaved = false;
    error = null;
    try {
      me = await apiFetch<MeResponse>('/users/me/settings', {
        method: 'PATCH',
        body: JSON.stringify({
          timezone: timezone.trim() ? timezone.trim() : null,
          metric_used: metric,
          hour_format: hourFormat,
          dark_mode: darkMode,
        }),
      });
      setThemePreference(darkMode);
      settingsSaved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to save settings';
    } finally {
      saving = false;
    }
  }
</script>

<Card.Root class="settings-card">
  <Card.Header>
    <Card.Title>Settings</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if loading}
      <div class="skeleton"></div>
    {:else if error && !me}
      <p class="error">{error}</p>
    {:else}
      <form on:submit|preventDefault={submit}>
        {#if error}<p class="error">{error}</p>{/if}
        {#if settingsSaved}<p class="success">Settings saved.</p>{/if}
        <label>
          Timezone
          <Input bind:value={timezone} placeholder="Europe/Paris" />
        </label>
        <label>
          Default metric
          <select bind:value={metric}>
            <option value="number">Count</option>
            <option value="duration">Duration</option>
          </select>
        </label>
        <label>
          Hour format
          <select bind:value={hourFormat}>
            <option value="24">24-hour</option>
            <option value="12">12-hour</option>
          </select>
        </label>
        <label>
          Theme
          <select bind:value={darkMode} on:change={() => setThemePreference(darkMode)}>
            <option value="follow">Follow system</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </label>
        <Button type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save settings'}</Button>
      </form>

      <section class="sharing">
        <div>
          <h3>Public sharing</h3>
          <p>Make Overview, History, Tracks, Artists, and Albums available through a secret public link.</p>
        </div>
        {#if publicSharingEnabled}
          {#if publicToken}
            <Input readonly value={publicLink} />
          {:else}
            <p class="muted">Public sharing is enabled. Rotate the link to reveal a new copyable URL.</p>
          {/if}
          <div class="actions">
            {#if publicToken}<Button type="button" variant="outline" disabled={sharingBusy} onclick={copyPublicLink}>{linkCopied ? 'Copied' : 'Copy link'}</Button>{/if}
            <Button type="button" variant="outline" disabled={sharingBusy} onclick={enableOrRotateSharing}>Rotate link</Button>
            <Button type="button" variant="destructive" disabled={sharingBusy} onclick={revokeSharing}>Revoke</Button>
          </div>
        {:else}
          <Button type="button" disabled={sharingBusy} onclick={enableOrRotateSharing}>{sharingBusy ? 'Enabling…' : 'Enable public sharing'}</Button>
        {/if}
      </section>

      <section class="sharing">
        <div>
          <h3>Spotify connection</h3>
          <p>Disconnecting deletes stored Spotify access and refresh tokens. Spotrak keeps existing history, but polling stops until you sign in with Spotify again.</p>
        </div>
        {#if spotifyDisconnected}<p class="success">Spotify disconnected. Sign in with Spotify again to reconnect.</p>{/if}
        {#if confirmDisconnect && !spotifyDisconnected}
          <p class="warning">This stops future Spotify polling until you reconnect. Existing history stays in Spotrak.</p>
          <div class="actions">
            <Button type="button" variant="destructive" disabled={spotifyBusy} onclick={disconnectSpotify}>
              {spotifyBusy ? 'Disconnecting…' : 'Confirm disconnect'}
            </Button>
            <Button type="button" variant="outline" disabled={spotifyBusy} onclick={() => (confirmDisconnect = false)}>Cancel</Button>
          </div>
        {:else}
          <Button type="button" variant="destructive" disabled={spotifyBusy || spotifyDisconnected} onclick={() => (confirmDisconnect = true)}>
            {spotifyDisconnected ? 'Disconnected' : 'Disconnect Spotify'}
          </Button>
        {/if}
      </section>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.settings-card) {
    max-width: 42rem;
  }

  form,
  .sharing {
    display: grid;
    gap: 0.9rem;
  }

  .sharing {
    margin-top: 1rem;
    border-top: 1px solid var(--color-border);
    padding-top: 1rem;
  }

  .sharing h3,
  .sharing p {
    margin: 0;
  }

  .sharing p {
    color: var(--color-muted);
    font-size: 0.84rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  label {
    display: grid;
    gap: 0.35rem;
    color: var(--color-muted);
    font-size: 0.82rem;
    font-weight: 700;
  }

  select {
    min-height: 2.25rem;
    padding: 0 0.6rem;
  }

  .success {
    color: var(--color-primary);
  }

  .warning {
    color: var(--color-muted);
  }

  .error {
    color: var(--color-danger);
  }

  .skeleton {
    min-height: 14rem;
  }
</style>
