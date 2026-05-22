<script lang="ts">
  import type { ListeningSessionStats, ListeningSessionSummary } from "../../../lib/api/types";
  import { formatDuration } from "../../../lib/date/format";
  import { formatDate, formatNumber } from "../../../lib/stats/insights";
  import CoverArt from "../../media/CoverArt.svelte";
  import * as Card from "../../ui/card";

  export let sessions: ListeningSessionStats | null = null;
  export let timezone: string | null = null;
  export let className = "";

  $: hasData = (sessions?.total_sessions ?? 0) > 0;

  function sessionTitle(session: ListeningSessionSummary | null | undefined): string {
    if (!session) return "No session";
    if (session.first_track_name === session.last_track_name) return session.first_track_name;
    return `${session.first_track_name} into ${session.last_track_name}`;
  }
</script>

<Card.Root class={`stats-card sessions-card ${className}`}>
  <Card.Header>
    <Card.Description>ten-minute gaps split sessions</Card.Description>
    <Card.Title>Listening sessions</Card.Title>
  </Card.Header>
  <Card.Content>
    {#if !hasData}
      <p class="state">No session data for this range.</p>
    {:else}
      <dl class="session-summary">
        <div>
          <dt>Sessions</dt>
          <dd>{formatNumber(sessions?.total_sessions)}</dd>
        </div>
        <div>
          <dt>Avg length</dt>
          <dd>{formatDuration(sessions?.average_duration_ms ?? 0)}</dd>
        </div>
        <div>
          <dt>Avg listens</dt>
          <dd>{formatNumber(sessions?.average_listens, 1)}</dd>
        </div>
      </dl>

      <div class="session-picks">
        {#if sessions?.longest}
          <article>
            <CoverArt src={sessions.longest.image_url} name={sessionTitle(sessions.longest)} size="sm" />
            <div>
              <span>Longest</span>
              <strong>{formatDuration(sessions.longest.duration_ms)}</strong>
              <p>{sessionTitle(sessions.longest)}</p>
              <small>{sessions.longest.listens} listens, {sessions.longest.unique_artists} artists, {formatDate(sessions.longest.start, timezone)}</small>
            </div>
          </article>
        {/if}

        {#if sessions?.most_intense}
          <article>
            <CoverArt src={sessions.most_intense.image_url} name={sessionTitle(sessions.most_intense)} size="sm" />
            <div>
              <span>Most intense</span>
              <strong>{formatNumber(sessions.most_intense.listens_per_hour, 1)}/h</strong>
              <p>{sessionTitle(sessions.most_intense)}</p>
              <small>{sessions.most_intense.listens} listens in {formatDuration(sessions.most_intense.duration_ms)}</small>
            </div>
          </article>
        {/if}
      </div>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.sessions-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.sessions-card [data-slot="card-content"]) {
    display: grid;
    gap: 0.85rem;
  }

  .session-summary {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.45rem;
    margin: 0;
  }

  .session-summary div {
    display: grid;
    gap: 0.16rem;
    border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.58rem;
    background: color-mix(in srgb, var(--color-panel) 42%, transparent);
  }

  .session-summary dt,
  .session-picks span {
    color: var(--color-muted);
    font-size: 0.68rem;
    font-weight: 780;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .session-summary dd {
    margin: 0;
    color: var(--color-text);
    font-size: 1rem;
    font-weight: 820;
    font-variant-numeric: tabular-nums;
  }

  .session-picks {
    display: grid;
    gap: 0.55rem;
  }

  .session-picks article {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.62rem;
    align-items: center;
    min-width: 0;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    padding-top: 0.62rem;
  }

  .session-picks article > div {
    display: grid;
    gap: 0.16rem;
    min-width: 0;
  }

  .session-picks strong {
    color: var(--color-text);
    font-size: 1.22rem;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .session-picks p,
  .session-picks small {
    overflow: hidden;
    margin: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-picks p {
    color: var(--color-text);
    font-weight: 760;
  }

  .session-picks small {
    color: var(--color-muted);
    font-size: 0.72rem;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }

  @media (max-width: 520px) {
    .session-summary {
      grid-template-columns: 1fr;
    }
  }
</style>
