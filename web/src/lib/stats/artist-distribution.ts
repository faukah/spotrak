import type { BucketedTopArtist } from "../api/types";

export const OTHER_ARTISTS_ID = "__other__";

export type DistributionEntity = {
  id: string;
  name: string;
  image_url?: string | null;
  total: number;
  isOther: boolean;
};

export type DistributionBucket = {
  bucket: string;
  total: number;
  values: Map<string, number>;
};

export type LanePoint = {
  bucket: string;
  label: string;
  value: number;
  bucketShare: number;
  height: number;
};

export type DistributionLane = {
  entity: DistributionEntity;
  color: string;
  share: number;
  points: LanePoint[];
  peak: LanePoint | null;
};

export function buildDistributionEntities(
  input: BucketedTopArtist[],
): DistributionEntity[] {
  const byId = new Map<string, DistributionEntity>();
  for (const row of input) {
    const isOther = row.id === OTHER_ARTISTS_ID;
    const current = byId.get(row.id) ?? {
      id: row.id,
      name: row.name,
      image_url: row.image_url,
      total: 0,
      isOther,
    };
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

export function buildDistributionBuckets(
  inputRows: BucketedTopArtist[],
  inputEntities: DistributionEntity[],
  inputBuckets: string[],
): DistributionBucket[] {
  const entityIds = new Set(inputEntities.map((entity) => entity.id));
  const byBucket = new Map<string, Map<string, number>>();
  for (const row of inputRows) {
    if (!entityIds.has(row.id)) continue;
    const bucket = byBucket.get(row.bucket) ?? new Map<string, number>();
    bucket.set(row.id, (bucket.get(row.id) ?? 0) + row.count);
    byBucket.set(row.bucket, bucket);
  }

  const bucketOrder = inputBuckets.length > 0
    ? inputBuckets
    : [...byBucket.keys()].toSorted((a, b) => a.localeCompare(b));

  return bucketOrder.map((bucket) => {
    const values = byBucket.get(bucket) ?? new Map<string, number>();
    const total = [...values.values()].reduce((sum, value) => sum + value, 0);
    return { bucket, total, values };
  });
}

export function buildDistributionLanes(
  inputEntities: DistributionEntity[],
  inputBuckets: DistributionBucket[],
  listenTotal: number,
  formatBucketLabel: (bucket: string) => string,
  laneColor: (index: number, entity: DistributionEntity) => string,
): DistributionLane[] {
  return inputEntities.map((entity, index) => {
    const rawPoints = inputBuckets.map((bucket) => {
      const value = bucket.values.get(entity.id) ?? 0;
      return {
        bucket: bucket.bucket,
        label: formatBucketLabel(bucket.bucket),
        value,
        bucketShare: bucket.total > 0 ? (value / bucket.total) * 100 : 0,
        height: 0,
      };
    });
    const maxValue = Math.max(1, ...rawPoints.map((point) => point.value));
    const points = rawPoints.map((point) => {
      point.height = sparkHeight(point.value, maxValue);
      return point;
    });
    const peak = points.toSorted((a, b) => b.value - a.value)[0] ?? null;

    return {
      entity,
      color: laneColor(index, entity),
      share: listenTotal > 0 ? (entity.total / listenTotal) * 100 : 0,
      points,
      peak: peak && peak.value > 0 ? peak : null,
    };
  });
}

function sparkHeight(value: number, maxValue: number): number {
  if (value <= 0) return 0;
  return Math.max(14, Math.min(100, (value / maxValue) * 100));
}
