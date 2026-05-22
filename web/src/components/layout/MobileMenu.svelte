<script lang="ts">
  import { onMount } from 'svelte';
  import AuthStatus from './AuthStatus.svelte';
  import SearchBox from '../search/SearchBox.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  type NavItem = {
    label: string;
    href: string;
    active?: boolean;
  };

  export let navItems: NavItem[] = [];
  export let includeSearch = false;
  export let includeAuth = false;
  export let menuId = 'mobile-navigation-menu';

  let open = false;
  let mounted = false;

  function close() {
    open = false;
  }

  function toggle() {
    open = !open;
  }

  onMount(() => {
    mounted = true;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };

    const desktopQuery = window.matchMedia('(min-width: 1501px)');
    const onMediaChange = () => {
      if (desktopQuery.matches) close();
    };

    window.addEventListener('keydown', onKeyDown);
    desktopQuery.addEventListener('change', onMediaChange);

    return () => {
      window.removeEventListener('keydown', onKeyDown);
      desktopQuery.removeEventListener('change', onMediaChange);
      document.body.classList.remove('mobile-menu-open');
    };
  });

  $: if (mounted) {
    document.body.classList.toggle('mobile-menu-open', open);
  }
</script>

<div class="mobile-menu">
  <button
    class="mobile-menu-button"
    type="button"
    aria-label={open ? 'Close navigation menu' : 'Open navigation menu'}
    aria-expanded={open}
    aria-controls={menuId}
    onclick={toggle}
  >
    <span class="mobile-menu-bars" class:open aria-hidden="true"></span>
  </button>

  {#if open}
    <button class="mobile-menu-backdrop" type="button" aria-label="Close navigation menu" onclick={close}></button>
  {/if}

  <div class="mobile-menu-panel" id={menuId} hidden={!open}>
    {#if includeSearch}
      <SearchBox id={`${menuId}-search`} name={`${menuId}-search`} />
    {/if}

    <nav class="mobile-menu-nav" aria-label="Mobile navigation">
      {#each navItems as item}
        <a href={item.href} aria-current={item.active ? 'page' : undefined} data-astro-prefetch="tap" onclick={close}>
          <span>{item.label}</span>
          <span class="mobile-menu-chevron" aria-hidden="true">›</span>
        </a>
      {/each}
    </nav>

    <div class="mobile-menu-footer">
      <ThemeToggle />
      {#if includeAuth}
        <AuthStatus />
      {/if}
    </div>
  </div>
</div>

<style>
  .mobile-menu {
    display: none;
  }

  @media (max-width: 1500px) {
    :global(body.mobile-menu-open) {
      overflow: hidden;
    }

    .mobile-menu {
      display: block;
      grid-column: 2;
      grid-row: 1;
      justify-self: end;
    }

    .mobile-menu-button {
      position: relative;
      z-index: 70;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 2.75rem;
      height: 2.75rem;
      border: 1px solid var(--color-border);
      border-radius: var(--radius-sm);
      background: color-mix(in srgb, var(--color-panel) 86%, transparent);
      color: var(--color-text);
      cursor: pointer;
      transition: border-color 140ms ease, background 140ms ease;
    }

    .mobile-menu-button:hover,
    .mobile-menu-button:focus-visible {
      border-color: color-mix(in srgb, var(--color-primary) 55%, var(--color-border));
      background: var(--color-panel-2);
      outline: none;
    }

    .mobile-menu-bars,
    .mobile-menu-bars::before,
    .mobile-menu-bars::after {
      display: block;
      width: 1.05rem;
      height: 2px;
      border-radius: 999px;
      background: currentColor;
      transition: transform 140ms ease, opacity 140ms ease, background 140ms ease;
    }

    .mobile-menu-bars {
      position: relative;
    }

    .mobile-menu-bars::before,
    .mobile-menu-bars::after {
      position: absolute;
      left: 0;
      content: "";
    }

    .mobile-menu-bars::before {
      transform: translateY(-0.36rem);
    }

    .mobile-menu-bars::after {
      transform: translateY(0.36rem);
    }

    .mobile-menu-bars.open {
      background: transparent;
    }

    .mobile-menu-bars.open::before {
      background: currentColor;
      transform: rotate(45deg);
    }

    .mobile-menu-bars.open::after {
      background: currentColor;
      transform: rotate(-45deg);
    }

    .mobile-menu-backdrop {
      position: fixed;
      inset: var(--mobile-header-height, 3.8rem) 0 0;
      z-index: 50;
      margin: 0;
      border: 0;
      border-radius: 0;
      padding: 0;
      background: color-mix(in srgb, var(--color-bg) 88%, transparent);
      cursor: default;
    }

    .mobile-menu-panel {
      position: fixed;
      top: calc(var(--mobile-header-height, 3.8rem) + 0.45rem);
      right: var(--space-page);
      left: var(--space-page);
      z-index: 60;
      display: grid;
      gap: 0.7rem;
      max-height: calc(100dvh - var(--mobile-header-height, 3.8rem) - 0.9rem);
      overflow: auto;
      border: 1px solid var(--color-border);
      border-radius: var(--radius-lg);
      padding: 0.75rem;
      background: color-mix(in srgb, var(--color-bg-elevated) 97%, var(--color-bg));
      box-shadow: var(--shadow-card);
      -webkit-overflow-scrolling: touch;
    }

    .mobile-menu-panel[hidden] {
      display: none;
    }

    .mobile-menu-nav {
      display: grid;
      gap: 0.35rem;
    }

    .mobile-menu-nav a {
      display: flex;
      align-items: center;
      justify-content: space-between;
      min-height: 2.75rem;
      border: 1px solid transparent;
      border-radius: var(--radius-sm);
      padding: 0.7rem 0.75rem;
      color: var(--color-muted);
      font-size: 0.95rem;
      font-weight: 760;
      text-decoration: none;
    }

    .mobile-menu-chevron {
      color: var(--color-muted);
      font-size: 1.1rem;
    }

    .mobile-menu-nav a:hover,
    .mobile-menu-nav a[aria-current="page"] {
      border-color: color-mix(in srgb, var(--color-border) 80%, var(--color-primary));
      background: color-mix(in srgb, var(--color-panel) 82%, transparent);
      color: var(--color-text);
    }

    .mobile-menu-footer {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.65rem;
      border-top: 1px solid color-mix(in srgb, var(--color-border) 84%, transparent);
      padding-top: 0.7rem;
    }
  }
</style>
