<script lang="ts">
  import { onMount } from 'svelte';
  import { applyThemePreference, getStoredThemePreference, setThemePreference, watchSystemTheme, type EffectiveTheme, type ThemePreference, type ThemeTransitionOrigin } from '../../lib/theme';

  const order: ThemePreference[] = ['follow', 'light', 'dark'];

  let preference: ThemePreference = 'follow';
  let theme: EffectiveTheme = 'light';
  let stopWatchingSystem: (() => void) | undefined;

  $: label = preference === 'follow' ? 'System' : preference === 'light' ? 'Light' : 'Dark';
  $: icon = preference === 'follow' ? '◐' : preference === 'light' ? '☼' : '☾';
  $: title = `Theme: ${label}${preference === 'follow' ? ` (${theme})` : ''}`;

  onMount(() => {
    preference = getStoredThemePreference();
    theme = applyThemePreference(preference);
    stopWatchingSystem = watchSystemTheme();

    const update = (event: Event) => {
      const detail = (event as CustomEvent<{ preference: ThemePreference; theme: EffectiveTheme }>).detail;
      preference = detail.preference;
      theme = detail.theme;
    };

    window.addEventListener('spotrak:theme-change', update);
    return () => {
      stopWatchingSystem?.();
      window.removeEventListener('spotrak:theme-change', update);
    };
  });

  function cycleTheme(event: MouseEvent) {
    const index = order.indexOf(preference);
    const next = order[(index + 1) % order.length] ?? 'follow';
    preference = next;
    theme = setThemePreference(next, themeTransitionOrigin(event));
  }

  function themeTransitionOrigin(event: MouseEvent): ThemeTransitionOrigin | undefined {
    if (!(event.currentTarget instanceof HTMLElement)) return undefined;
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
    };
  }
</script>

<button class="theme-toggle" type="button" aria-label={`${title}. Change theme`} {title} onclick={cycleTheme}>
  {#key icon}
    <span aria-hidden="true" class="theme-icon">{icon}</span>
  {/key}
  <span class="theme-label">{label}</span>
</button>

<style>
  .theme-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    min-height: 2rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 0.6rem;
    background: color-mix(in srgb, var(--color-panel) 82%, transparent);
    color: var(--color-text);
    font-size: 0.82rem;
    font-weight: 750;
    white-space: nowrap;
    cursor: pointer;
    transition:
      border-color var(--motion-feedback) var(--ease-out-quart),
      background var(--motion-feedback) var(--ease-out-quart),
      color var(--motion-feedback) var(--ease-out-quart),
      transform var(--motion-feedback) var(--ease-out-quart);
  }

  .theme-toggle:hover {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--color-border));
    background: var(--color-panel-2);
  }

  .theme-toggle:active {
    transform: translateY(1px);
  }

  .theme-icon {
    width: 1em;
    color: var(--color-primary);
    text-align: center;
    animation: theme-icon-settle var(--motion-state) var(--ease-out-quart) both;
  }

  @keyframes theme-icon-settle {
    from {
      opacity: 0;
      transform: rotate(-18deg) scale(0.86);
    }
  }

  @media (max-width: 500px) {
    .theme-toggle {
      width: 2.75rem;
      min-height: 2.75rem;
      padding: 0;
    }

    .theme-label {
      display: none;
    }
  }

  @media (pointer: coarse) {
    .theme-toggle {
      min-width: 2.75rem;
      min-height: 2.75rem;
    }
  }
</style>
