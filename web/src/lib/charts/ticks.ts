type BandTickStrideOptions = {
  minTickSpacing: number;
  maxTicks?: number;
  minTicks?: number;
  horizontalPadding?: number;
};

export function bandTickStride(
  itemCount: number,
  width: number,
  {
    minTickSpacing,
    maxTicks = itemCount,
    minTicks = 2,
    horizontalPadding = 0,
  }: BandTickStrideOptions,
): number {
  if (!Number.isFinite(itemCount) || itemCount <= 1) return 1;

  const safeWidth = Number.isFinite(width) ? width : 0;
  const plotWidth = Math.max(0, safeWidth - horizontalPadding);
  const ticksByWidth = plotWidth > 0 ? Math.floor(plotWidth / minTickSpacing) : maxTicks;
  const floorTicks = Math.min(itemCount, Math.max(1, minTicks));
  const targetTicks = Math.min(itemCount, maxTicks, Math.max(floorTicks, ticksByWidth));

  return Math.max(1, Math.ceil(itemCount / Math.max(1, targetTicks)));
}
