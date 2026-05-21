<script lang="ts">
  import { scaleUtc } from 'd3-scale';
  import { curveMonotoneX } from 'd3-shape';
  import { AreaChart } from 'layerchart';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import type { BucketedTopArtist } from '../../lib/api/types';
  import { chartColor, formatCountValue, formatPercentValue, numericValue } from '../../lib/charts/theme';
  import { directImageUrl, transitionHref, viewTransitionName } from '../../lib/images';
  import CoverArt from '../media/CoverArt.svelte';
  import * as Card from '../ui/card';
  import * as Chart from '../ui/chart';

  type DistributionEntity = {
    id: string;
    name: string;
    image_url?: string | null;
    total: number;
    isOther: boolean;
  };

  type DistributionSegment = DistributionEntity & {
    value: number;
    percent: number;
    color: string;
  };

  type DistributionBucket = {
    bucket: string;
    total: number;
    segments: DistributionSegment[];
  };

  type DistributionDatum = {
    bucket: string;
    date: Date;
    total: number;
  } & Record<string, Date | string | number>;

  type TooltipItem = { color?: string };
  type TooltipPayloadValue = { value?: unknown };

  export let rows: BucketedTopArtist[] = [];
  export let bucketKeys: string[] = [];
  export let timelineDescription = 'monthly buckets';
  export let pagePrefix = '';
  export let formatBucketLabel: (bucket: string) => string = (bucket) => bucket;
  export let className = '';

  const OTHER_ARTISTS_ID = '__other__';

  let prefersReducedMotion = false;
  let stopReducedMotionWatch: (() => void) | undefined;

  onMount(() => {
    stopReducedMotionWatch = watchReducedMotion();
    return () => stopReducedMotionWatch?.();
  });

  $: entities = buildDistributionEntities(rows);
  $: buckets = buildDistributionBuckets(rows, entities, bucketKeys);
  $: chartBuckets = buckets.filter((bucket) => bucket.total > 0);
  $: chartData = buildDistributionChartData(chartBuckets, entities);
  $: chartConfig = Object.fromEntries(
    entities.map((entity, index) => [entity.id, { label: entity.name, color: distributionColor(index, entity) }]),
  ) satisfies Chart.ChartConfig;
  $: series = entities.map((entity, index) => ({
    key: entity.id,
    label: entity.name,
    value: entity.id,
    color: distributionColor(index, entity),
    props: distributionAreaProps(entity),
  }));
  $: tickCount = Math.min(8, Math.max(2, chartData.length));
  $: chartMotionKey = distributionChartKey(chartData, entities);

  function buildDistributionEntities(input: BucketedTopArtist[]): DistributionEntity[] {
    const byId = new Map<string, DistributionEntity>();
    for (const row of input) {
      const isOther = row.id === OTHER_ARTISTS_ID;
      const current = byId.get(row.id) ?? { id: row.id, name: row.name, image_url: row.image_url, total: 0, isOther };
      current.total += row.count;
      current.image_url ||= row.image_url;
      current.isOther ||= isOther;
      byId.set(row.id, current);
    }
    return [...byId.values()].toSorted((a, b) => {
      if (a.isOther !== b.isOther) return a.isOther ? 1 : -1;
      return b.total - a.total;
    });
  }

  function buildDistributionBuckets(inputRows: BucketedTopArtist[], inputEntities: DistributionEntity[], inputBuckets: string[]): DistributionBucket[] {
    const entityIds = new Set(inputEntities.map((entity) => entity.id));
    const byBucket = new Map<string, Map<string, number>>();
    for (const row of inputRows) {
      if (!entityIds.has(row.id)) continue;
      const bucket = byBucket.get(row.bucket) ?? new Map<string, number>();
      bucket.set(row.id, (bucket.get(row.id) ?? 0) + row.count);
      byBucket.set(row.bucket, bucket);
    }

    return inputBuckets.map((bucket) => {
      const values = byBucket.get(bucket) ?? new Map<string, number>();
      const total = [...values.values()].reduce((sum, value) => sum + value, 0);
      const segments = inputEntities.map((entity, index) => {
        const value = values.get(entity.id) ?? 0;
        return {
          ...entity,
          value,
          percent: total > 0 ? (value / total) * 100 : 0,
          color: distributionColor(index, entity),
        };
      });
      return { bucket, total, segments };
    });
  }

  function buildDistributionChartData(inputBuckets: DistributionBucket[], inputEntities: DistributionEntity[]): DistributionDatum[] {
    return inputBuckets.map((bucket) => {
      const date = parseBucketDate(bucket.bucket) ?? new Date(bucket.bucket);
      const datum: DistributionDatum = { bucket: bucket.bucket, date, total: bucket.total };
      for (const entity of inputEntities) datum[entity.id] = distributionSegmentValue(bucket, entity.id);
      return datum;
    });
  }

  function distributionChartKey(inputData: DistributionDatum[], inputEntities: DistributionEntity[]): string {
    const entityKey = inputEntities.map((entity) => entity.id).join(',');
    const valueKey = inputData
      .map((datum) => `${datum.bucket}:${datum.total}:${inputEntities.map((entity) => Number(datum[entity.id] ?? 0)).join(',')}`)
      .join('|');
    return `${entityKey}::${valueKey}`;
  }

  function watchReducedMotion(): () => void {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => {
      prefersReducedMotion = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }

  function motionDuration(duration: number): number {
    return prefersReducedMotion ? 1 : duration;
  }

  function parseBucketDate(value: string): Date | null {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  function distributionSegmentValue(bucket: DistributionBucket, entityId: string): number {
    return bucket.segments.find((segment) => segment.id === entityId)?.value ?? 0;
  }

  function distributionColor(index: number, entity?: DistributionEntity): string {
    return entity?.isOther ? 'var(--color-muted)' : chartColor(index);
  }

  function distributionAreaProps(entity: DistributionEntity) {
    if (entity.isOther) {
      return {
        role: 'presentation',
        tabindex: -1,
        'aria-hidden': true,
      };
    }

    return {
      onclick: () => openArtist(entity.id),
      onkeydown: (event: KeyboardEvent) => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        event.preventDefault();
        openArtist(entity.id);
      },
      role: 'link',
      tabindex: 0,
      'aria-label': `Open artist ${entity.name}`,
    };
  }

  function openArtist(id: string) {
    window.location.assign(artistHref(id));
  }

  function artistHref(id: string): string {
    return `${pagePrefix}/artist/${id}`;
  }

  function artistTransition(id: string, scope: string): string {
    return viewTransitionName(id, scope);
  }

  function artistTransitionHref(id: string, scope: string): string {
    return transitionHref(artistHref(id), artistTransition(id, scope));
  }

  function formatDistributionValue(value: number): string {
    return `${value.toLocaleString()} listens`;
  }

  function formatTooltipLabel(value: unknown): string {
    if (value instanceof Date) return formatDateLabel(value);
    return formatBucketLabel(String(value ?? ''));
  }

  function formatDateLabel(date: Date): string {
    return formatBucketLabel(`${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:00:00`);
  }

  function pad(value: number): string {
    return String(value).padStart(2, '0');
  }

  function formatTooltipValue(value: unknown, payload: TooltipPayloadValue[]): string {
    const raw = numericValue(value);
    const total = payload.reduce((sum, item) => sum + numericValue(item.value), 0);
    const share = total > 0 ? (raw / total) * 100 : 0;
    return `${formatCountValue(raw)} listens, ${formatPercentValue(share)}`;
  }

  function formatDistributionCell(bucket: DistributionBucket, entityId: string): string {
    const segment = bucket.segments.find((item) => item.id === entityId);
    const value = segment?.value ?? 0;
    const percent = segment?.percent ?? 0;
    return `${formatDistributionValue(value)}, ${formatPercentValue(percent)}`;
  }

  function hasTooltipValue(item: TooltipPayloadValue): boolean {
    return numericValue(item.value) > 0;
  }

  function tooltipColor(item: TooltipItem): string {
    return item.color ?? 'currentColor';
  }
</script>

<Card.Root class={`stats-card artist-distribution-card ${className} overflow-visible`}>
  <Card.Header>
    <Card.Description>top artists plus Other artists by {timelineDescription}</Card.Description>
    <Card.Title>Artist listening distribution</Card.Title>
  </Card.Header>
  <Card.Content class="artist-distribution-content">
    {#if chartBuckets.length === 0}
      <p class="state">Not enough artist distribution data for this range.</p>
    {:else}
      <div class="distribution-legend" aria-label="Artist distribution legend">
        {#each entities as entity, index (entity.id)}
          {#if entity.isOther}
            <span
              class="legend-other"
              in:fade={{ duration: motionDuration(120) }}
              out:fade={{ duration: motionDuration(80) }}
              style={`--swatch: ${distributionColor(index, entity)};`}
              title={`${entity.name}: ${formatDistributionValue(entity.total)}`}
            >
              <span class="legend-swatch" aria-hidden="true"></span>
              <span>{entity.name}</span>
            </span>
          {:else}
            <a
              href={artistTransitionHref(entity.id, `stats-distribution-${index}`)}
              in:fade={{ duration: motionDuration(120) }}
              out:fade={{ duration: motionDuration(80) }}
              style={`--swatch: ${distributionColor(index, entity)};`}
              title={`${entity.name}: ${formatDistributionValue(entity.total)}`}
            >
              <CoverArt src={directImageUrl(entity)} name={entity.name} size="xs" transitionName={artistTransition(entity.id, `stats-distribution-${index}`)} />
              <span>{entity.name}</span>
            </a>
          {/if}
        {/each}
      </div>
      <div class="stacked-area-frame">
        {#key chartMotionKey}
          <div class="chart-reveal">
            <Chart.Container
              config={chartConfig}
              class="artist-distribution-chart"
              role="group"
              aria-label={`Top artist listen share by ${timelineDescription}, with the rest grouped as Other artists. Artist areas open detail pages.`}
            >
              <AreaChart
                data={chartData}
                x="date"
                xScale={scaleUtc()}
                {series}
                seriesLayout="stackExpand"
                padding={{ left: 46, right: 18, top: 14, bottom: 38 }}
                grid={{ y: true }}
                props={{
                  xAxis: { format: formatTooltipLabel, ticks: tickCount },
                  yAxis: { format: (value: unknown) => formatPercentValue(numericValue(value) * 100), ticks: 4 },
                  area: {
                    curve: curveMonotoneX,
                    fillOpacity: 0.68,
                    line: { strokeWidth: 1.1, motion: { type: 'none' }, role: 'presentation', tabindex: -1, 'aria-hidden': true, 'aria-label': undefined },
                    motion: { type: 'none' },
                  },
                  tooltip: { context: { mode: 'quadtree-x' } },
                }}
              >
                {#snippet tooltip()}
                  <Chart.Tooltip contained="window" class="distribution-tooltip" labelFormatter={formatTooltipLabel} filter={hasTooltipValue} indicator="line">
                    {#snippet formatter({ value, name, item, payload })}
                      <div class="tooltip-row">
                        <span class="tooltip-swatch" style:background={tooltipColor(item)}></span>
                        <span class="tooltip-name">{name}</span>
                        <span class="tooltip-value">{formatTooltipValue(value, payload)}</span>
                      </div>
                    {/snippet}
                  </Chart.Tooltip>
                {/snippet}
              </AreaChart>
            </Chart.Container>
          </div>
        {/key}
      </div>
      <table class="sr-only">
        <caption>Artist listening distribution data</caption>
        <thead>
          <tr>
            <th scope="col">Time bucket</th>
            <th scope="col">Artist</th>
            <th scope="col">Share</th>
          </tr>
        </thead>
        <tbody>
          {#each chartBuckets as bucket (bucket.bucket)}
            {#each bucket.segments.filter((segment) => segment.value > 0) as segment (segment.id)}
              <tr>
                <td>{formatBucketLabel(bucket.bucket)}</td>
                <td>{segment.name}</td>
                <td>{formatDistributionCell(bucket, segment.id)}</td>
              </tr>
            {/each}
          {/each}
        </tbody>
      </table>
    {/if}
  </Card.Content>
</Card.Root>

<style>
  :global(.artist-distribution-card) {
    min-width: 0;
    height: 100%;
  }

  :global(.artist-distribution-content) {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
  }

  .distribution-legend {
    display: flex;
    gap: 0.3rem;
    overflow-x: auto;
    padding-bottom: 0.35rem;
    scrollbar-width: thin;
  }

  .distribution-legend a,
  .distribution-legend .legend-other {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    max-width: 11rem;
    border: 1px solid color-mix(in srgb, var(--swatch) 44%, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.45rem 0.25rem 0.25rem;
    background: color-mix(in srgb, var(--swatch) 12%, transparent);
    color: var(--color-text);
    font-size: 0.7rem;
    font-weight: 700;
    text-decoration: none;
  }

  .distribution-legend .legend-other {
    color: var(--color-muted);
  }

  .legend-swatch {
    width: 1.35rem;
    height: 1.35rem;
    flex: 0 0 auto;
    border: 1px solid color-mix(in srgb, var(--swatch) 52%, var(--color-border));
    border-radius: var(--radius-xs);
    background: color-mix(in srgb, var(--swatch) 72%, transparent);
  }

  .distribution-legend a > span:last-child,
  .distribution-legend .legend-other > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stacked-area-frame {
    flex: 1;
    min-height: 0;
    min-width: 0;
    padding: 0.2rem 0 0.1rem;
  }

  .chart-reveal {
    min-width: 0;
    height: 100%;
    animation: distribution-chart-in 180ms var(--ease-out-quint) both;
  }

  @keyframes distribution-chart-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.artist-distribution-chart) {
    width: 100%;
    min-width: 0;
    height: 20.5rem;
    aspect-ratio: auto;
  }

  :global(.artist-distribution-chart .lc-area-path) {
    cursor: pointer;
    opacity: 0.92;
  }

  :global(.artist-distribution-chart .lc-area-path[aria-hidden='true']) {
    cursor: default;
  }

  :global(.artist-distribution-chart .lc-area-path:focus-visible) {
    outline: 2px solid color-mix(in srgb, var(--color-primary) 70%, transparent);
    outline-offset: 2px;
  }

  :global(.artist-distribution-chart .lc-spline-path) {
    pointer-events: none;
    stroke-linejoin: round;
  }

  :global(.distribution-tooltip) {
    max-height: min(20rem, calc(100vh - 2rem));
    overflow: auto;
  }

  .tooltip-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-width: 13rem;
  }

  .tooltip-swatch {
    width: 0.58rem;
    height: 0.58rem;
    border-radius: 999px;
  }

  .tooltip-name {
    overflow: hidden;
    color: var(--color-text);
    font-weight: 750;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip-value {
    color: var(--color-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .state {
    margin: 0;
    color: var(--color-muted);
  }
</style>
