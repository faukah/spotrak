use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        stats::{BucketedTopAlbum, BucketedTopArtist, BucketedTopTrack},
        time::{Metric, TimeSplit},
    },
    error::Result,
};

use super::{bucket_grain, metric_order_expression};

#[allow(
    clippy::too_many_arguments,
    reason = "bucket queries are thin repository wrappers around API filters"
)]
pub async fn top_tracks_by_bucket(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit_per_bucket: i64,
) -> Result<Vec<BucketedTopTrack>> {
    let grain = bucket_grain(split);
    let order = metric_order_expression(metric);
    let sql = format!(
        r#"
        WITH ranked AS (
          SELECT to_char(date_trunc($3, timezone($2, le.played_at)), 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
                 t.id, t.name, a.id AS album_id, a.name AS album_name,
                 ar.id AS artist_id, ar.name AS artist_name,
                 COALESCE(t.images->0->>'url', a.images->0->>'url') AS image_url,
                 COUNT(*)::bigint AS count,
                 COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms,
                 ROW_NUMBER() OVER (
                   PARTITION BY date_trunc($3, timezone($2, le.played_at))
                   ORDER BY {order} DESC, t.name ASC, t.id ASC
                 ) AS rank
          FROM listening_events le
          JOIN tracks t ON t.id = le.track_id
          JOIN albums a ON a.id = le.album_id
          JOIN artists ar ON ar.id = le.primary_artist_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($4::timestamptz IS NULL OR le.played_at >= $4)
            AND ($5::timestamptz IS NULL OR le.played_at < $5)
          GROUP BY date_trunc($3, timezone($2, le.played_at)), t.id, t.name, a.id, a.name, ar.id, ar.name, t.images, a.images
        )
        SELECT bucket, id, name, album_id, album_name, artist_id, artist_name, image_url, count, duration_ms
        FROM ranked
        WHERE rank <= $6
        ORDER BY bucket ASC, rank ASC, name ASC, id ASC
        "#
    );
    let rows = sqlx::query_as::<_, BucketedTopTrack>(&sql)
        .bind(user_id)
        .bind(timezone)
        .bind(grain)
        .bind(start)
        .bind(end)
        .bind(limit_per_bucket)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[allow(
    clippy::too_many_arguments,
    reason = "bucket queries are thin repository wrappers around API filters"
)]
pub async fn top_artists_by_bucket(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit_per_bucket: i64,
) -> Result<Vec<BucketedTopArtist>> {
    let grain = bucket_grain(split);
    let order = metric_order_expression(metric);
    let sql = format!(
        r#"
        WITH ranked AS (
          SELECT to_char(date_trunc($3, timezone($2, le.played_at)), 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
                 ar.id, ar.name,
                 COALESCE(
                   ar.images->0->>'url',
                   (array_remove(array_agg(COALESCE(t.images->0->>'url', a.images->0->>'url') ORDER BY le.played_at DESC, le.id DESC), NULL))[1]
                 ) AS image_url,
                 COUNT(*)::bigint AS count,
                 COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms,
                 ROW_NUMBER() OVER (
                   PARTITION BY date_trunc($3, timezone($2, le.played_at))
                   ORDER BY {order} DESC, ar.name ASC, ar.id ASC
                 ) AS rank
          FROM listening_events le
          JOIN artists ar ON ar.id = le.primary_artist_id
          JOIN tracks t ON t.id = le.track_id
          JOIN albums a ON a.id = le.album_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($4::timestamptz IS NULL OR le.played_at >= $4)
            AND ($5::timestamptz IS NULL OR le.played_at < $5)
          GROUP BY date_trunc($3, timezone($2, le.played_at)), ar.id, ar.name, ar.images
        )
        SELECT bucket, id, name, image_url, count, duration_ms
        FROM ranked
        WHERE rank <= $6
        ORDER BY bucket ASC, rank ASC, name ASC, id ASC
        "#
    );
    let rows = sqlx::query_as::<_, BucketedTopArtist>(&sql)
        .bind(user_id)
        .bind(timezone)
        .bind(grain)
        .bind(start)
        .bind(end)
        .bind(limit_per_bucket)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[allow(
    clippy::too_many_arguments,
    reason = "bucket queries are thin repository wrappers around API filters"
)]
pub async fn top_artists_by_bucket_with_other(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    artist_limit: i64,
) -> Result<Vec<BucketedTopArtist>> {
    let grain = bucket_grain(split);
    let order = match metric {
        Metric::Count => "listen_count",
        Metric::Duration => "total_duration_ms",
    };
    let sql = format!(
        r#"
        WITH filtered AS (
          SELECT date_trunc($3, timezone($2, le.played_at)) AS bucket_date,
                 le.primary_artist_id AS artist_id,
                 le.duration_ms,
                 le.played_at,
                 le.id AS event_id,
                 t.images AS track_images,
                 a.images AS album_images
          FROM listening_events le
          JOIN tracks t ON t.id = le.track_id
          JOIN albums a ON a.id = le.album_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($4::timestamptz IS NULL OR le.played_at >= $4)
            AND ($5::timestamptz IS NULL OR le.played_at < $5)
        ),
        artist_totals AS (
          SELECT ar.id,
                 ar.name,
                 COALESCE(
                   ar.images->0->>'url',
                   (array_remove(array_agg(COALESCE(f.track_images->0->>'url', f.album_images->0->>'url') ORDER BY f.played_at DESC, f.event_id DESC), NULL))[1]
                 ) AS image_url,
                 COUNT(*)::bigint AS listen_count,
                 COALESCE(SUM(f.duration_ms), 0)::bigint AS total_duration_ms
          FROM filtered f
          JOIN artists ar ON ar.id = f.artist_id
          GROUP BY ar.id, ar.name, ar.images
        ),
        top_artists AS (
          SELECT *
          FROM (
            SELECT artist_totals.*,
                   ROW_NUMBER() OVER (ORDER BY {order} DESC, name ASC, id ASC) AS rank
            FROM artist_totals
          ) ranked
          WHERE rank <= $6
        ),
        bucketed AS (
          SELECT to_char(f.bucket_date, 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
                 COALESCE(top_artists.id, '__other__') AS id,
                 COALESCE(top_artists.name, 'Other artists') AS name,
                 CASE WHEN top_artists.id IS NULL THEN NULL ELSE top_artists.image_url END AS image_url,
                 COUNT(*)::bigint AS count,
                 COALESCE(SUM(f.duration_ms), 0)::bigint AS duration_ms,
                 COALESCE(top_artists.rank, $6 + 1) AS rank
          FROM filtered f
          LEFT JOIN top_artists ON top_artists.id = f.artist_id
          GROUP BY f.bucket_date, top_artists.id, top_artists.name, top_artists.image_url, top_artists.rank
        )
        SELECT bucket, id, name, image_url, count, duration_ms
        FROM bucketed
        WHERE count > 0
        ORDER BY bucket ASC, rank ASC, name ASC, id ASC
        "#
    );
    let rows = sqlx::query_as::<_, BucketedTopArtist>(&sql)
        .bind(user_id)
        .bind(timezone)
        .bind(grain)
        .bind(start)
        .bind(end)
        .bind(artist_limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[allow(
    clippy::too_many_arguments,
    reason = "bucket queries are thin repository wrappers around API filters"
)]
pub async fn top_albums_by_bucket(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit_per_bucket: i64,
) -> Result<Vec<BucketedTopAlbum>> {
    let grain = bucket_grain(split);
    let order = metric_order_expression(metric);
    let sql = format!(
        r#"
        WITH ranked AS (
          SELECT to_char(date_trunc($3, timezone($2, le.played_at)), 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
                 a.id, a.name, MIN(ar.name) AS artist_name,
                 a.images->0->>'url' AS image_url,
                 COUNT(*)::bigint AS count,
                 COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms,
                 ROW_NUMBER() OVER (
                   PARTITION BY date_trunc($3, timezone($2, le.played_at))
                   ORDER BY {order} DESC, a.name ASC, a.id ASC
                 ) AS rank
          FROM listening_events le
          JOIN albums a ON a.id = le.album_id
          LEFT JOIN LATERAL (
            SELECT artist_id
            FROM album_artists
            WHERE album_id = a.id
            ORDER BY position ASC, artist_id ASC
            LIMIT 1
          ) aa ON TRUE
          LEFT JOIN artists ar ON ar.id = aa.artist_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($4::timestamptz IS NULL OR le.played_at >= $4)
            AND ($5::timestamptz IS NULL OR le.played_at < $5)
          GROUP BY date_trunc($3, timezone($2, le.played_at)), a.id, a.name, a.images
        )
        SELECT bucket, id, name, artist_name, image_url, count, duration_ms
        FROM ranked
        WHERE rank <= $6
        ORDER BY bucket ASC, rank ASC, name ASC, id ASC
        "#
    );
    let rows = sqlx::query_as::<_, BucketedTopAlbum>(&sql)
        .bind(user_id)
        .bind(timezone)
        .bind(grain)
        .bind(start)
        .bind(end)
        .bind(limit_per_bucket)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
