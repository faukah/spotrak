<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { apiFetch, loginUrl } from '../../lib/api/client';
  import type { CurrentlyPlayingResponse } from '../../lib/api/types';
  import * as Card from '../ui/card';

  export let endpoint = '/player/currently-playing';
  export let showReconnect = false;
  export let variant: 'header' | 'card' = 'header';
  export let pagePrefix = '';

  const PLAYING_POLL_MS = 15_000;
  const PAUSED_POLL_MS = 30_000;
  const IDLE_POLL_MS = 45_000;
  const ERROR_POLL_MS = 60_000;

  let playback: CurrentlyPlayingResponse | null = null;
  let pollTimer: number | undefined;
  let progressTimer: number | undefined;
  let marqueeFrame: number | undefined;
  let progressNow = Date.now();
  let loaded = false;
  let loadFailed = false;
  let mounted = false;
  let reconnectUrl = loginUrl();
  let lineElement: HTMLSpanElement | null = null;
  let marqueeContentElement: HTMLSpanElement | null = null;
  let marqueeOverflow = false;
  let marqueeDistance = 0;
  let marqueeDuration = '12s';

  $: track = playback?.track ?? null;
  $: progressMs = currentProgressMs(playback, progressNow);
  $: progressPercent = track?.duration_ms
    ? Math.min(100, Math.max(0, (progressMs / track.duration_ms) * 100))
    : 0;
  $: statusLabel = playback?.is_playing ? 'Currently' : 'Paused';
  $: cardStatusLabel = playback?.is_playing ? 'Now playing' : 'Paused';
  $: trackHref = track ? `${pagePrefix}/track/${track.id}` : undefined;

  onMount(() => {
    mounted = true;
    reconnectUrl = loginUrl(`${window.location.pathname}${window.location.search}${window.location.hash}`);
    const handleVisibilityChange = () => {
      if (document.hidden) {
        clearPollTimer();
        return;
      }
      void loadPlayback();
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    if (variant === 'header') window.addEventListener('resize', queueMarqueeMeasure);
    void loadPlayback();
    progressTimer = window.setInterval(() => {
      if (!mounted) return;
      progressNow = Date.now();
    }, 1_000);

    return () => {
      mounted = false;
      clearPollTimer();
      clearMarqueeFrame();
      if (progressTimer !== undefined) window.clearInterval(progressTimer);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (variant === 'header') window.removeEventListener('resize', queueMarqueeMeasure);
    };
  });

  async function loadPlayback() {
    clearPollTimer();
    if (!mounted || document.hidden) return;

    try {
      const nextPlayback = await apiFetch<CurrentlyPlayingResponse>(endpoint, { cache: 'no-store' });
      if (!mounted) return;
      playback = nextPlayback;
      loadFailed = false;
      progressNow = Date.now();
      if (variant === 'header') queueMarqueeMeasure();
      schedulePoll(nextPollMs(playback));
    } catch {
      if (!mounted) return;
      loadFailed = true;
      schedulePoll(ERROR_POLL_MS);
    } finally {
      if (mounted) loaded = true;
    }
  }

  function nextPollMs(value: CurrentlyPlayingResponse | null): number {
    if (!value?.track) return IDLE_POLL_MS;
    return value.is_playing ? PLAYING_POLL_MS : PAUSED_POLL_MS;
  }

  function schedulePoll(delay: number) {
    if (!mounted) return;
    pollTimer = window.setTimeout(() => void loadPlayback(), delay);
  }

  function clearPollTimer() {
    if (pollTimer !== undefined) window.clearTimeout(pollTimer);
    pollTimer = undefined;
  }

  async function queueMarqueeMeasure() {
    if (!mounted || variant !== 'header') return;
    await tick();
    if (!mounted || variant !== 'header') return;
    clearMarqueeFrame();
    marqueeFrame = window.requestAnimationFrame(() => {
      if (!mounted) return;
      marqueeFrame = undefined;
      measureMarquee();
    });
  }

  function clearMarqueeFrame() {
    if (marqueeFrame !== undefined) window.cancelAnimationFrame(marqueeFrame);
    marqueeFrame = undefined;
  }

  function measureMarquee() {
    if (!lineElement || !marqueeContentElement) {
      marqueeOverflow = false;
      marqueeDistance = 0;
      return;
    }

    const overflow = marqueeContentElement.scrollWidth - lineElement.clientWidth;
    marqueeOverflow = overflow > 1;
    marqueeDistance = marqueeContentElement.scrollWidth + 32;
    marqueeDuration = `${Math.max(12, Math.min(24, marqueeDistance / 18)).toFixed(2)}s`;
  }

  function currentProgressMs(value: CurrentlyPlayingResponse | null, now: number): number {
    if (!value?.track) return 0;
    const base = value.progress_ms ?? 0;
    if (!value.is_playing) return base;
    const fetchedAt = Date.parse(value.fetched_at);
    if (!Number.isFinite(fetchedAt)) return base;
    return Math.min(value.track.duration_ms, Math.max(0, base + now - fetchedAt));
  }

  function formatPlaybackTime(value: number): string {
    const totalSeconds = Math.max(0, Math.floor(value / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    if (hours > 0) {
      return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }
    return `${minutes}:${String(seconds).padStart(2, '0')}`;
  }

</script>

{#if variant === 'card'}
  <Card.Root class="now-playing-card" size="sm">
    <Card.Header>
      <Card.Description>{track ? cardStatusLabel : 'Spotify player'}</Card.Description>
      <Card.Title>Currently playing</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if track}
        <section class="now-playing-panel" aria-label={`${statusLabel} playing: ${track.name} from ${track.album_name}`}>
          <a class="now-playing-art" href={trackHref} aria-label={track.name}>
            {#if track.image_url}
              <img src={track.image_url} alt="" loading="lazy" decoding="async" />
            {:else}
              <span class="art-placeholder large" aria-hidden="true"></span>
            {/if}
          </a>
          <div class="now-playing-copy">
            <a class="now-playing-title" href={trackHref}>{track.name}</a>
            <span class="now-playing-artist">{track.artist_name ?? 'Unknown artist'}</span>
            <span class="now-playing-album">{track.album_name}</span>
          </div>
          <div class="now-playing-progress" aria-label={`${formatPlaybackTime(progressMs)} of ${formatPlaybackTime(track.duration_ms)}`}>
            <div>
              <span>{formatPlaybackTime(progressMs)}</span>
              <span>{formatPlaybackTime(track.duration_ms)}</span>
            </div>
            <div class="progress-track large" aria-hidden="true">
              <span style={`width: ${progressPercent}%;`}></span>
            </div>
          </div>
        </section>
      {:else if !loaded}
        <div class="now-playing-state" aria-live="polite">
          <span class="art-placeholder large" aria-hidden="true"></span>
          <div>
            <strong>Checking Spotify playback</strong>
            <p>Waiting for the player endpoint.</p>
          </div>
        </div>
      {:else if showReconnect && playback?.unavailable_reason === 'reconnect_required'}
        <a class="now-playing-state reconnect-card" href={reconnectUrl}>
          <span class="reconnect-mark large" aria-hidden="true"></span>
          <div>
            <strong>Reconnect Spotify playback</strong>
            <p>The player token needs a fresh login.</p>
          </div>
        </a>
      {:else}
        <div class="now-playing-state" aria-live="polite">
          <span class="art-placeholder large" aria-hidden="true"></span>
          <div>
            <strong>{loadFailed ? 'Playback unavailable' : 'Nothing playing'}</strong>
            <p>{loadFailed ? 'The player endpoint did not respond.' : 'Start Spotify playback and this card will update.'}</p>
          </div>
        </div>
      {/if}
    </Card.Content>
  </Card.Root>
{:else if track}
  <section class="currently-playing" aria-label={`${statusLabel} playing: ${track.name} from ${track.album_name}`}>
    {#if track.image_url}
      <img src={track.image_url} alt="" loading="lazy" decoding="async" />
    {:else}
      <span class="art-placeholder" aria-hidden="true"></span>
    {/if}
    <div class="playback-copy">
      <span
        class="playback-line playback-text"
        class:marquee={marqueeOverflow}
        bind:this={lineElement}
        style={`--marquee-distance: ${marqueeDistance}px; --marquee-duration: ${marqueeDuration};`}
      >
        <span class="marquee-runner">
          <span class="marquee-content" bind:this={marqueeContentElement}>
            <span class="playback-title">{track.name}</span>
            <span class="playback-separator" aria-hidden="true">·</span>
            <span class="playback-album">{track.album_name}</span>
          </span>
          {#if marqueeOverflow}
            <span class="marquee-content marquee-copy" aria-hidden="true">
              <span class="playback-title">{track.name}</span>
              <span class="playback-separator" aria-hidden="true">·</span>
              <span class="playback-album">{track.album_name}</span>
            </span>
          {/if}
        </span>
        <span class="sr-only">
          <span class="playback-title">{track.name}</span>
          <span class="playback-separator" aria-hidden="true">·</span>
          <span class="playback-album">{track.album_name}</span>
        </span>
      </span>
    </div>
    <div class="progress-track" aria-hidden="true">
      <span style={`width: ${progressPercent}%;`}></span>
    </div>
  </section>
{:else if showReconnect && playback?.unavailable_reason === 'reconnect_required'}
  <a class="currently-playing reconnect" href={reconnectUrl}>
    <span class="reconnect-mark" aria-hidden="true"></span>
    <span>Reconnect for live playback</span>
  </a>
{/if}

<style>
  .currently-playing {
    position: relative;
    display: grid;
    grid-template-columns: 1.9rem minmax(0, 1fr);
    gap: 0.48rem;
    align-items: center;
    width: min(15rem, 18vw);
    min-width: 9rem;
    height: 2.35rem;
    overflow: hidden;
    border-radius: var(--radius-xs);
    padding-inline: 0.25rem;
    background: transparent;
    color: var(--color-text);
    text-decoration: none;
  }

  .currently-playing img,
  .currently-playing .art-placeholder,
  .currently-playing .reconnect-mark {
    width: 1.9rem;
    height: 1.9rem;
    margin-left: 0;
    border-radius: calc(var(--radius-xs) - 1px);
    background: var(--color-panel-2);
    object-fit: cover;
  }

  .playback-copy {
    display: grid;
    min-width: 0;
  }

  .currently-playing .reconnect-mark {
    background: color-mix(in srgb, var(--color-primary) 68%, var(--color-panel-2));
  }

  .playback-text {
    display: block;
    min-width: 0;
    overflow: hidden;
    line-height: 1.1;
    white-space: nowrap;
  }

  .playback-line {
    font-size: 0.8rem;
    -webkit-mask-image: linear-gradient(90deg, black 0, black calc(100% - 0.8rem), transparent 100%);
    mask-image: linear-gradient(90deg, black 0, black calc(100% - 0.8rem), transparent 100%);
  }

  .marquee-runner,
  .marquee-content {
    display: inline-flex;
    align-items: baseline;
    white-space: nowrap;
  }

  .marquee-runner {
    min-width: 100%;
  }

  .playback-line.marquee .marquee-runner {
    gap: 2rem;
    animation: playback-marquee var(--marquee-duration) linear infinite;
  }

  .playback-line:not(.marquee) .marquee-copy {
    display: none;
  }

  .playback-title {
    color: var(--color-text);
    font-weight: 740;
  }

  .playback-separator {
    color: color-mix(in srgb, var(--color-muted) 76%, transparent);
    padding-inline: 0.32rem;
  }

  .playback-album {
    color: var(--color-muted);
    font-weight: 560;
  }

  .progress-track {
    position: absolute;
    right: 0.25rem;
    bottom: 0.22rem;
    left: 2.65rem;
    height: 1px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-border) 42%, transparent);
  }

  .progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: color-mix(in srgb, var(--color-primary) 58%, var(--color-text));
    transition: width 900ms linear;
  }

  .reconnect {
    grid-template-columns: 0.65rem minmax(0, 1fr);
    min-width: 10rem;
    padding-inline: 0.7rem;
    color: var(--color-muted);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .currently-playing .reconnect-mark {
    width: 0.5rem;
    height: 0.5rem;
    margin: 0;
    border-radius: 999px;
  }

  :global(.now-playing-card) {
    min-width: 0;
    height: 100%;
    gap: 0.85rem;
    padding-block: 1rem;
  }

  :global(.now-playing-card [data-slot='card-header']),
  :global(.now-playing-card [data-slot='card-content']) {
    padding-inline: 1rem;
  }

  :global(.now-playing-card [data-slot='card-content']) {
    display: grid;
    height: 100%;
  }

  :global(.now-playing-panel) {
    display: grid;
    grid-template-columns: minmax(6.5rem, 8.5rem) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.8rem 0.9rem;
    min-width: 0;
    height: 100%;
    align-items: center;
  }

  :global(.now-playing-art) {
    grid-row: 1 / 3;
    display: block;
    min-width: 0;
    color: inherit;
    text-decoration: none;
  }

  :global(.now-playing-art img),
  :global(.art-placeholder.large) {
    width: 100%;
    height: auto;
    aspect-ratio: 1;
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--color-border) 86%, transparent);
    border-radius: var(--radius-xs);
    object-fit: cover;
  }

  :global(.now-playing-copy) {
    display: grid;
    min-width: 0;
    gap: 0.25rem;
    align-self: end;
  }

  :global(.now-playing-title) {
    overflow: hidden;
    color: var(--color-text);
    font-size: clamp(1.12rem, 1.8vw, 1.55rem);
    font-weight: 850;
    line-height: 1.05;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.now-playing-title:hover) {
    color: var(--color-primary);
  }

  :global(.now-playing-artist) {
    overflow: hidden;
    color: var(--color-text);
    font-size: 0.9rem;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.now-playing-album) {
    overflow: hidden;
    color: var(--color-muted);
    font-size: 0.82rem;
    font-weight: 680;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.now-playing-progress) {
    display: grid;
    gap: 0.32rem;
    align-self: end;
  }

  :global(.now-playing-progress > div:first-child) {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    color: var(--color-muted);
    font-size: 0.7rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  :global(.now-playing-progress .progress-track.large) {
    position: static;
    width: 100%;
    height: 0.32rem;
    background: color-mix(in srgb, var(--color-border) 62%, transparent);
  }

  :global(.now-playing-state) {
    display: grid;
    grid-template-columns: 4.6rem minmax(0, 1fr);
    gap: 0.8rem;
    align-items: center;
    min-height: 8.5rem;
    color: inherit;
    text-decoration: none;
  }

  :global(.now-playing-state strong) {
    color: var(--color-text);
    font-size: 1.05rem;
    line-height: 1.1;
  }

  :global(.now-playing-state p) {
    margin: 0.25rem 0 0;
    color: var(--color-muted);
    font-size: 0.82rem;
    font-weight: 650;
  }

  :global(.reconnect-card:hover strong) {
    color: var(--color-primary);
  }

  :global(.reconnect-mark.large) {
    width: 4.6rem;
    height: 4.6rem;
    border-radius: var(--radius-xs);
  }

  @keyframes playback-marquee {
    0%,
    16% {
      transform: translateX(0);
    }

    100% {
      transform: translateX(calc(-1 * var(--marquee-distance)));
    }
  }

  @media (max-width: 1500px) {
    .currently-playing {
      width: min(18rem, calc(100vw - (2 * var(--space-page)) - 3.7rem));
      min-width: 0;
    }
  }

  @media (max-width: 420px) {
    .currently-playing {
      grid-template-columns: 1.8rem minmax(0, 1fr);
      height: 2.3rem;
    }

    .currently-playing img,
    .currently-playing .art-placeholder {
      width: 1.8rem;
      height: 1.8rem;
    }

    .progress-track {
      left: 2.5rem;
    }

    :global(.now-playing-panel) {
      grid-template-columns: 4.8rem minmax(0, 1fr);
      gap: 0.65rem 0.75rem;
    }

    :global(.now-playing-title) {
      font-size: 1rem;
    }

    :global(.now-playing-artist),
    :global(.now-playing-album) {
      font-size: 0.78rem;
    }

    :global(.now-playing-state) {
      grid-template-columns: 3.8rem minmax(0, 1fr);
      min-height: 6.5rem;
    }

    :global(.reconnect-mark.large) {
      width: 3.8rem;
      height: 3.8rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .playback-line.marquee .marquee-runner {
      animation: none;
    }

    .progress-track span {
      transition-duration: 0.001ms;
    }
  }
</style>
