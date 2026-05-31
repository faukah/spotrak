use std::collections::{HashMap, HashSet};

use crate::db::{PgPool, Transaction};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        stats::{
            AlbumReleaseYearPoint, AlbumReleaseYearsStats, ArtistRunSummary, ComebackArtist,
            DiscoveryStats, DiversityTimelinePoint, EntityStats, FeatureAverageStats,
            FeatureRatioStats, FeatureTimelinePoint, HistoryEvent, HourRepartitionPoint,
            HourlyTopArtist, ListeningConcentrationStats, ListeningSessionStats,
            ListeningSessionSummary, LongestSession, RepeatLoopStats, RepeatLoopSummary,
            SummaryStats, TimelinePoint, TopAlbum, TopArtist, TopTrack,
        },
        time::{Metric, TimeSplit},
    },
    error::Result,
};

mod bucketed;

pub use bucketed::{
    top_albums_by_bucket, top_artists_by_bucket, top_artists_by_bucket_with_other,
    top_tracks_by_bucket,
};

pub async fn history(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEvent>> {
    let rows = crate::db::query_as::<HistoryEvent>(
        r#"
        SELECT le.id,
               t.id AS track_id,
               t.name AS track_name,
               a.id AS album_id,
               a.name AS album_name,
               ar.id AS artist_id,
               ar.name AS artist_name,
               COALESCE(t.images->0->>'url', a.images->0->>'url') AS image_url,
               le.duration_ms,
               le.played_at,
               le.source
        FROM listening_events le
        JOIN tracks t ON t.id = le.track_id
        JOIN albums a ON a.id = le.album_id
        JOIN artists ar ON ar.id = le.primary_artist_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        ORDER BY le.played_at DESC, le.id DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn summary(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<SummaryStats> {
    let stats = crate::db::query_as::<SummaryStats>(
        r#"
        SELECT COUNT(*)::bigint AS total_listens,
               COALESCE(SUM(duration_ms), 0)::bigint AS total_duration_ms,
               COUNT(DISTINCT track_id)::bigint AS unique_tracks,
               COUNT(DISTINCT primary_artist_id)::bigint AS unique_artists,
               COUNT(DISTINCT album_id)::bigint AS unique_albums
        FROM listening_events
        WHERE user_id = $1
          AND blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR played_at >= $2)
          AND ($3::timestamptz IS NULL OR played_at < $3)
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    Ok(stats)
}

pub async fn discovery_stats(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<DiscoveryStats> {
    let stats = crate::db::query_as::<DiscoveryStats>(
        r#"
        WITH range_events AS MATERIALIZED (
          SELECT track_id, primary_artist_id, played_at
          FROM listening_events
          WHERE user_id = $1
            AND blacklisted_by IS NULL
            AND ($2::timestamptz IS NULL OR played_at >= $2)
            AND ($3::timestamptz IS NULL OR played_at < $3)
        ),
        range_tracks AS (
          SELECT track_id, MIN(played_at) AS first_in_range
          FROM range_events
          GROUP BY track_id
        ),
        range_artists AS (
          SELECT primary_artist_id, MIN(played_at) AS first_in_range
          FROM range_events
          GROUP BY primary_artist_id
        ),
        first_tracks AS (
          SELECT rt.track_id, MIN(le.played_at) AS first_played_at
          FROM range_tracks rt
          JOIN listening_events le
            ON le.user_id = $1
           AND le.blacklisted_by IS NULL
           AND le.track_id = rt.track_id
          GROUP BY rt.track_id
        ),
        first_artists AS (
          SELECT ra.primary_artist_id, MIN(le.played_at) AS first_played_at
          FROM range_artists ra
          JOIN listening_events le
            ON le.user_id = $1
           AND le.blacklisted_by IS NULL
           AND le.primary_artist_id = ra.primary_artist_id
          GROUP BY ra.primary_artist_id
        ),
        counts AS (
          SELECT
            (SELECT COUNT(*) FROM range_events)::bigint AS total_listens,
            (SELECT COUNT(*) FROM range_tracks)::bigint AS unique_tracks,
            (SELECT COUNT(*) FROM range_artists)::bigint AS unique_artists,
            (
              SELECT COUNT(*)
              FROM range_tracks rt
              JOIN first_tracks ft ON ft.track_id = rt.track_id
              WHERE ft.first_played_at = rt.first_in_range
            )::bigint AS new_tracks,
            (
              SELECT COUNT(*)
              FROM range_artists ra
              JOIN first_artists fa ON fa.primary_artist_id = ra.primary_artist_id
              WHERE fa.first_played_at = ra.first_in_range
            )::bigint AS new_artists
        )
        SELECT total_listens,
               unique_tracks,
               unique_artists,
               new_tracks,
               new_artists,
               GREATEST(total_listens - new_tracks, 0)::bigint AS repeat_listens,
               CASE WHEN total_listens > 0
                 THEN (new_tracks::float8 / total_listens::float8) * 100
                 ELSE 0
               END AS discovery_share,
               CASE WHEN total_listens > 0
                 THEN (GREATEST(total_listens - new_tracks, 0)::float8 / total_listens::float8) * 100
                 ELSE 0
               END AS repeat_share
        FROM counts
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    Ok(stats)
}

pub async fn timeline(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<TimelinePoint>> {
    let grain = match split {
        TimeSplit::All => None,
        TimeSplit::Year => Some("year"),
        TimeSplit::Month => Some("month"),
        TimeSplit::Week => Some("week"),
        TimeSplit::Day => Some("day"),
        TimeSplit::Hour => Some("hour"),
    };

    if let Some(grain) = grain {
        let rows = crate::db::query_as::<TimelinePoint>(
            r#"
            SELECT to_char(date_trunc($3, timezone($2, played_at)), 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
                   COUNT(*)::bigint AS count,
                   COALESCE(SUM(duration_ms), 0)::bigint AS duration_ms
            FROM listening_events
            WHERE user_id = $1
              AND blacklisted_by IS NULL
              AND ($4::timestamptz IS NULL OR played_at >= $4)
              AND ($5::timestamptz IS NULL OR played_at < $5)
            GROUP BY date_trunc($3, timezone($2, played_at))
            ORDER BY date_trunc($3, timezone($2, played_at)) ASC
            "#,
        )
        .bind(user_id)
        .bind(timezone)
        .bind(grain)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    } else {
        let row = crate::db::query_as::<TimelinePoint>(
            r#"
            SELECT 'all'::text AS bucket,
                   COUNT(*)::bigint AS count,
                   COALESCE(SUM(duration_ms), 0)::bigint AS duration_ms
            FROM listening_events
            WHERE user_id = $1
              AND blacklisted_by IS NULL
              AND ($2::timestamptz IS NULL OR played_at >= $2)
              AND ($3::timestamptz IS NULL OR played_at < $3)
            "#,
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(pool)
        .await?;
        Ok(vec![row])
    }
}

pub async fn top_tracks(
    pool: &PgPool,
    user_id: Uuid,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TopTrack>> {
    let order = metric_order(metric);
    let sql = format!(
        r#"
        SELECT t.id, t.name, a.id AS album_id, a.name AS album_name,
               ar.id AS artist_id, ar.name AS artist_name,
               COALESCE(t.images->0->>'url', a.images->0->>'url') AS image_url,
               COUNT(*)::bigint AS count,
               COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms
        FROM listening_events le
        JOIN tracks t ON t.id = le.track_id
        JOIN albums a ON a.id = le.album_id
        JOIN artists ar ON ar.id = le.primary_artist_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        GROUP BY t.id, t.name, a.id, a.name, ar.id, ar.name, t.images, a.images
        ORDER BY {order} DESC, t.name ASC, t.id ASC
        LIMIT $4 OFFSET $5
        "#
    );
    let rows = crate::db::query_as::<TopTrack>(&sql)
        .bind(user_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn top_artists(
    pool: &PgPool,
    user_id: Uuid,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TopArtist>> {
    let order = metric_order(metric);
    let sql = format!(
        r#"
        SELECT ar.id, ar.name,
               COALESCE(
                 ar.images->0->>'url',
                 (array_remove(array_agg(COALESCE(t.images->0->>'url', al.images->0->>'url') ORDER BY le.played_at DESC, le.id DESC), NULL))[1]
               ) AS image_url,
               COUNT(*)::bigint AS count,
               COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms
        FROM listening_events le
        JOIN artists ar ON ar.id = le.primary_artist_id
        JOIN tracks t ON t.id = le.track_id
        JOIN albums al ON al.id = le.album_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        GROUP BY ar.id, ar.name, ar.images
        ORDER BY {order} DESC, ar.name ASC, ar.id ASC
        LIMIT $4 OFFSET $5
        "#
    );
    let rows = crate::db::query_as::<TopArtist>(&sql)
        .bind(user_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn concentration_stats(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<ListeningConcentrationStats> {
    let stats = crate::db::query_as::<ListeningConcentrationStats>(
        r#"
        WITH artist_counts AS (
          SELECT ar.id,
                 ar.name,
                 COALESCE(
                   ar.images->0->>'url',
                   (array_remove(array_agg(COALESCE(t.images->0->>'url', al.images->0->>'url') ORDER BY le.played_at DESC, le.id DESC), NULL))[1]
                 ) AS image_url,
                 COUNT(*)::bigint AS listen_count
          FROM listening_events le
          JOIN artists ar ON ar.id = le.primary_artist_id
          JOIN tracks t ON t.id = le.track_id
          JOIN albums al ON al.id = le.album_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($2::timestamptz IS NULL OR le.played_at >= $2)
            AND ($3::timestamptz IS NULL OR le.played_at < $3)
          GROUP BY ar.id, ar.name, ar.images
        ),
        ranked AS (
          SELECT *,
                 ROW_NUMBER() OVER (ORDER BY listen_count DESC, name ASC, id ASC) AS rank
          FROM artist_counts
        ),
        totals AS (
          SELECT COALESCE(SUM(listen_count), 0)::bigint AS total_listens,
                 COUNT(*)::bigint AS artist_count,
                 COALESCE(SUM(CASE WHEN rank <= 5 THEN listen_count ELSE 0 END), 0)::bigint AS top_five_listens,
                 COALESCE(SUM(CASE WHEN rank <= 10 THEN listen_count ELSE 0 END), 0)::bigint AS top_ten_listens,
                 COALESCE(SUM((listen_count::float8 * listen_count::float8)), 0)::float8 AS listen_squares
          FROM ranked
        ),
        top_artist AS (
          SELECT id, name, image_url, listen_count
          FROM ranked
          WHERE rank = 1
        )
        SELECT totals.total_listens,
               totals.artist_count,
               top_artist.id AS top_artist_id,
               top_artist.name AS top_artist_name,
               top_artist.image_url AS top_artist_image_url,
               COALESCE(top_artist.listen_count, 0)::bigint AS top_artist_listens,
               CASE WHEN totals.total_listens > 0
                 THEN (COALESCE(top_artist.listen_count, 0)::float8 / totals.total_listens::float8) * 100
                 ELSE 0
               END AS top_artist_share,
               CASE WHEN totals.total_listens > 0
                 THEN (totals.top_five_listens::float8 / totals.total_listens::float8) * 100
                 ELSE 0
               END AS top_five_share,
               CASE WHEN totals.total_listens > 0
                 THEN (totals.top_ten_listens::float8 / totals.total_listens::float8) * 100
                 ELSE 0
               END AS top_ten_share,
               CASE WHEN totals.listen_squares > 0
                 THEN (totals.total_listens::float8 * totals.total_listens::float8) / totals.listen_squares
                 ELSE 0
               END AS effective_artist_count
        FROM totals
        LEFT JOIN top_artist ON TRUE
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    Ok(stats)
}

pub async fn comeback_artists(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<ComebackArtist>> {
    let rows = crate::db::query_as::<ComebackArtist>(
        r#"
        WITH artist_events AS MATERIALIZED (
          SELECT le.primary_artist_id,
                 le.played_at,
                 LAG(le.played_at) OVER (
                   PARTITION BY le.primary_artist_id
                   ORDER BY le.played_at ASC, le.id ASC
                 ) AS previous_played_at
          FROM listening_events le
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($3::timestamptz IS NULL OR le.played_at < $3)
        ),
        returns AS (
          SELECT primary_artist_id,
                 played_at AS returned_at,
                 previous_played_at,
                 (EXTRACT(EPOCH FROM (played_at - previous_played_at)) * 1000)::bigint AS gap_ms,
                 ROW_NUMBER() OVER (
                   PARTITION BY primary_artist_id
                   ORDER BY played_at - previous_played_at DESC, played_at ASC
                 ) AS rank
          FROM artist_events
          WHERE previous_played_at IS NOT NULL
            AND ($2::timestamptz IS NULL OR played_at >= $2)
            AND ($3::timestamptz IS NULL OR played_at < $3)
        ),
        artist_range_counts AS (
          SELECT primary_artist_id, COUNT(*)::bigint AS range_listens
          FROM listening_events
          WHERE user_id = $1
            AND blacklisted_by IS NULL
            AND ($2::timestamptz IS NULL OR played_at >= $2)
            AND ($3::timestamptz IS NULL OR played_at < $3)
          GROUP BY primary_artist_id
        )
        SELECT r.primary_artist_id AS artist_id,
               ar.name AS artist_name,
               COALESCE(ar.images->0->>'url', latest_image.image_url) AS image_url,
               r.gap_ms,
               r.previous_played_at,
               r.returned_at,
               COALESCE(arc.range_listens, 0)::bigint AS range_listens
        FROM returns r
        JOIN artists ar ON ar.id = r.primary_artist_id
        LEFT JOIN artist_range_counts arc ON arc.primary_artist_id = r.primary_artist_id
        LEFT JOIN LATERAL (
          SELECT COALESCE(t.images->0->>'url', al.images->0->>'url') AS image_url
          FROM listening_events le
          JOIN tracks t ON t.id = le.track_id
          JOIN albums al ON al.id = le.album_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND le.primary_artist_id = r.primary_artist_id
            AND le.played_at <= r.returned_at
          ORDER BY le.played_at DESC, le.id DESC
          LIMIT 1
        ) latest_image ON TRUE
        WHERE r.rank = 1
        ORDER BY r.gap_ms DESC, ar.name ASC, r.primary_artist_id ASC
        LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn top_artists_by_hour(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit_per_hour: i64,
) -> Result<Vec<HourlyTopArtist>> {
    let order = metric_order_expression(metric);
    let sql = format!(
        r#"
        WITH ranked AS (
          SELECT EXTRACT(HOUR FROM timezone($2, le.played_at))::int AS hour,
                 ar.id AS artist_id,
                 ar.name AS artist_name,
                 COALESCE(
                   ar.images->0->>'url',
                   (array_remove(array_agg(COALESCE(t.images->0->>'url', a.images->0->>'url') ORDER BY le.played_at DESC, le.id DESC), NULL))[1]
                 ) AS image_url,
                 COUNT(*)::bigint AS count,
                 COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms,
                 ROW_NUMBER() OVER (
                   PARTITION BY EXTRACT(HOUR FROM timezone($2, le.played_at))::int
                   ORDER BY {order} DESC, ar.name ASC, ar.id ASC
                 ) AS rank
          FROM listening_events le
          JOIN artists ar ON ar.id = le.primary_artist_id
          JOIN tracks t ON t.id = le.track_id
          JOIN albums a ON a.id = le.album_id
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($3::timestamptz IS NULL OR le.played_at >= $3)
            AND ($4::timestamptz IS NULL OR le.played_at < $4)
          GROUP BY EXTRACT(HOUR FROM timezone($2, le.played_at))::int, ar.id, ar.name, ar.images
        )
        SELECT hour, artist_id, artist_name, image_url, count, duration_ms, rank
        FROM ranked
        WHERE rank <= $5
        ORDER BY hour ASC, rank ASC, artist_name ASC, artist_id ASC
        "#
    );
    let rows = crate::db::query_as::<HourlyTopArtist>(&sql)
        .bind(user_id)
        .bind(timezone)
        .bind(start)
        .bind(end)
        .bind(limit_per_hour)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn top_albums(
    pool: &PgPool,
    user_id: Uuid,
    metric: Metric,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TopAlbum>> {
    let order = metric_order(metric);
    let sql = format!(
        r#"
        SELECT a.id, a.name, MIN(ar.name) AS artist_name, a.release_year,
               a.images->0->>'url' AS image_url,
               COUNT(*)::bigint AS count,
               COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms
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
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        GROUP BY a.id, a.name, a.release_year, a.images
        ORDER BY {order} DESC, a.name ASC, a.id ASC
        LIMIT $4 OFFSET $5
        "#
    );
    let rows = crate::db::query_as::<TopAlbum>(&sql)
        .bind(user_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

fn metric_order(metric: Metric) -> &'static str {
    match metric {
        Metric::Count => "count",
        Metric::Duration => "duration_ms",
    }
}

pub async fn hour_repartition(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<HourRepartitionPoint>> {
    let rows = crate::db::query_as::<HourRepartitionPoint>(
        r#"
        SELECT EXTRACT(HOUR FROM timezone($2, played_at))::int AS hour,
               COUNT(*)::bigint AS count,
               COALESCE(SUM(duration_ms), 0)::bigint AS duration_ms
        FROM listening_events
        WHERE user_id = $1
          AND blacklisted_by IS NULL
          AND ($3::timestamptz IS NULL OR played_at >= $3)
          AND ($4::timestamptz IS NULL OR played_at < $4)
        GROUP BY EXTRACT(HOUR FROM timezone($2, played_at))::int
        ORDER BY hour ASC
        "#,
    )
    .bind(user_id)
    .bind(timezone)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn feature_ratio(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<FeatureRatioStats> {
    let stats = crate::db::query_as::<FeatureRatioStats>(
        r#"
        WITH track_artist_counts AS (
          SELECT track_id, COUNT(*)::bigint AS artist_count
          FROM track_artists
          GROUP BY track_id
        )
        SELECT
          COUNT(*) FILTER (WHERE COALESCE(tac.artist_count, 1) <= 1)::bigint AS solo_count,
          COUNT(*) FILTER (WHERE COALESCE(tac.artist_count, 1) > 1)::bigint AS feature_count,
          COALESCE(SUM(le.duration_ms) FILTER (WHERE COALESCE(tac.artist_count, 1) <= 1), 0)::bigint AS solo_duration_ms,
          COALESCE(SUM(le.duration_ms) FILTER (WHERE COALESCE(tac.artist_count, 1) > 1), 0)::bigint AS feature_duration_ms
        FROM listening_events le
        LEFT JOIN track_artist_counts tac ON tac.track_id = le.track_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    Ok(stats)
}

pub async fn feature_average(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<FeatureAverageStats> {
    let stats = crate::db::query_as::<FeatureAverageStats>(
        r#"
        WITH listened_tracks AS (
          SELECT DISTINCT le.track_id
          FROM listening_events le
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($2::timestamptz IS NULL OR le.played_at >= $2)
            AND ($3::timestamptz IS NULL OR le.played_at < $3)
        ),
        track_feature_counts AS (
          SELECT lt.track_id,
                 GREATEST(COUNT(ta.artist_id) - 1, 0)::bigint AS feature_count
          FROM listened_tracks lt
          LEFT JOIN track_artists ta ON ta.track_id = lt.track_id
          GROUP BY lt.track_id
        )
        SELECT COUNT(*)::bigint AS unique_tracks,
               COUNT(*) FILTER (WHERE feature_count > 0)::bigint AS featured_tracks,
               COALESCE(SUM(feature_count), 0)::bigint AS total_features,
               COALESCE(AVG(feature_count::float8), 0)::float8 AS average_features_per_song
        FROM track_feature_counts
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;
    Ok(stats)
}

pub async fn feature_timeline(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<FeatureTimelinePoint>> {
    let grain = bucket_grain(split);
    let rows = crate::db::query_as::<FeatureTimelinePoint>(
        r#"
        WITH listened_tracks AS (
          SELECT date_trunc($3, timezone($2, le.played_at)) AS bucket,
                 le.track_id
          FROM listening_events le
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND ($4::timestamptz IS NULL OR le.played_at >= $4)
            AND ($5::timestamptz IS NULL OR le.played_at < $5)
          GROUP BY date_trunc($3, timezone($2, le.played_at)), le.track_id
        ),
        track_feature_counts AS (
          SELECT lt.bucket,
                 lt.track_id,
                 GREATEST(COUNT(ta.artist_id) - 1, 0)::bigint AS feature_count
          FROM listened_tracks lt
          LEFT JOIN track_artists ta ON ta.track_id = lt.track_id
          GROUP BY lt.bucket, lt.track_id
        )
        SELECT to_char(bucket, 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
               COUNT(*)::bigint AS unique_tracks,
               COUNT(*) FILTER (WHERE feature_count > 0)::bigint AS featured_tracks,
               COALESCE(SUM(feature_count), 0)::bigint AS total_features,
               COALESCE(AVG(feature_count::float8), 0)::float8 AS average_features_per_song
        FROM track_feature_counts
        GROUP BY bucket
        ORDER BY bucket ASC
        "#,
    )
    .bind(user_id)
    .bind(timezone)
    .bind(grain)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn album_release_years(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<AlbumReleaseYearsStats> {
    let distribution = crate::db::query_as::<AlbumReleaseYearPoint>(
        r#"
        SELECT a.release_year,
               COUNT(*)::bigint AS count,
               COALESCE(SUM(le.duration_ms), 0)::bigint AS duration_ms
        FROM listening_events le
        JOIN albums a ON a.id = le.album_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        GROUP BY a.release_year
        ORDER BY a.release_year ASC NULLS LAST
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    let weighted_years = distribution
        .iter()
        .filter_map(|point| {
            point
                .release_year
                .map(|year| (year as f64, point.count as f64))
        })
        .collect::<Vec<_>>();
    let total_weight = weighted_years.iter().map(|(_, count)| count).sum::<f64>();
    let average_release_year = if total_weight > 0.0 {
        Some(
            weighted_years
                .iter()
                .map(|(year, count)| year * count)
                .sum::<f64>()
                / total_weight,
        )
    } else {
        None
    };

    Ok(AlbumReleaseYearsStats {
        average_release_year,
        distribution,
    })
}

pub async fn diversity_timeline(
    pool: &PgPool,
    user_id: Uuid,
    timezone: &str,
    split: TimeSplit,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<DiversityTimelinePoint>> {
    let grain = bucket_grain(split);
    let rows = crate::db::query_as::<DiversityTimelinePoint>(
        r#"
        SELECT to_char(date_trunc($3, timezone($2, le.played_at)), 'YYYY-MM-DD"T"HH24:MI:SS') AS bucket,
               COUNT(DISTINCT le.track_id)::bigint AS unique_tracks,
               COUNT(DISTINCT le.primary_artist_id)::bigint AS unique_artists,
               COUNT(DISTINCT le.album_id)::bigint AS unique_albums,
               AVG(a.release_year::float8) AS average_release_year
        FROM listening_events le
        JOIN albums a ON a.id = le.album_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($4::timestamptz IS NULL OR le.played_at >= $4)
          AND ($5::timestamptz IS NULL OR le.played_at < $5)
        GROUP BY date_trunc($3, timezone($2, le.played_at))
        ORDER BY date_trunc($3, timezone($2, le.played_at)) ASC
        "#,
    )
    .bind(user_id)
    .bind(timezone)
    .bind(grain)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn entity_stats(
    pool: &PgPool,
    user_id: Uuid,
    entity: EntityFilter<'_>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<EntityStats> {
    let column = match entity {
        EntityFilter::Track(_) => "track_id",
        EntityFilter::Artist(_) => "primary_artist_id",
        EntityFilter::Album(_) => "album_id",
    };
    let id = entity.id();
    let sql = format!(
        r#"
        SELECT COUNT(*)::bigint AS total_listens,
               COALESCE(SUM(duration_ms), 0)::bigint AS total_duration_ms,
               COUNT(DISTINCT track_id)::bigint AS unique_tracks,
               COUNT(DISTINCT primary_artist_id)::bigint AS unique_artists,
               COUNT(DISTINCT album_id)::bigint AS unique_albums,
               MIN(played_at) AS first_played_at,
               MAX(played_at) AS last_played_at
        FROM listening_events
        WHERE user_id = $1
          AND blacklisted_by IS NULL
          AND {column} = $2
          AND ($3::timestamptz IS NULL OR played_at >= $3)
          AND ($4::timestamptz IS NULL OR played_at < $4)
        "#
    );
    let stats = crate::db::query_as::<EntityStats>(&sql)
        .bind(user_id)
        .bind(id)
        .bind(start)
        .bind(end)
        .fetch_one(pool)
        .await?;
    Ok(stats)
}

pub async fn delete_imported_history(tx: &Transaction<'_>, user_id: Uuid) -> Result<u64> {
    let result = crate::db::query(
        r#"
        DELETE FROM listening_events
        WHERE user_id = $1
          AND source IN ('privacy-import', 'full-privacy-import')
        "#,
    )
    .bind(user_id)
    .execute(tx)
    .await?;
    Ok(result.rows_affected())
}

pub enum EntityFilter<'a> {
    Track(&'a str),
    Artist(&'a str),
    Album(&'a str),
}

impl EntityFilter<'_> {
    fn id(&self) -> &str {
        match self {
            EntityFilter::Track(id) | EntityFilter::Artist(id) | EntityFilter::Album(id) => id,
        }
    }
}

pub async fn longest_sessions(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<LongestSession>> {
    let events = history_ascending(pool, user_id, start, end).await?;
    Ok(build_longest_sessions(&events, limit))
}

pub async fn listening_behavior_stats(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<(ListeningSessionStats, RepeatLoopStats)> {
    let events = history_ascending(pool, user_id, start, end).await?;
    let sessions = build_sessions(&events);
    Ok((session_stats(&sessions), repeat_loop_stats(&events)))
}

async fn history_ascending(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<HistoryEvent>> {
    let rows = crate::db::query_as::<HistoryEvent>(
        r#"
        SELECT le.id,
               t.id AS track_id,
               t.name AS track_name,
               a.id AS album_id,
               a.name AS album_name,
               ar.id AS artist_id,
               ar.name AS artist_name,
               COALESCE(t.images->0->>'url', a.images->0->>'url') AS image_url,
               le.duration_ms,
               le.played_at,
               le.source
        FROM listening_events le
        JOIN tracks t ON t.id = le.track_id
        JOIN albums a ON a.id = le.album_id
        JOIN artists ar ON ar.id = le.primary_artist_id
        WHERE le.user_id = $1
          AND le.blacklisted_by IS NULL
          AND ($2::timestamptz IS NULL OR le.played_at >= $2)
          AND ($3::timestamptz IS NULL OR le.played_at < $3)
        ORDER BY le.played_at ASC, le.id ASC
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

fn build_longest_sessions(events: &[HistoryEvent], limit: i64) -> Vec<LongestSession> {
    let mut sessions = build_sessions(events);
    sessions.sort_by(|a, b| {
        b.duration_ms
            .cmp(&a.duration_ms)
            .then_with(|| a.start.cmp(&b.start))
            .then_with(|| a.end.cmp(&b.end))
    });
    sessions.truncate(limit as usize);
    sessions
}

fn build_sessions(events: &[HistoryEvent]) -> Vec<LongestSession> {
    if events.is_empty() {
        return Vec::new();
    }

    let mut sessions = Vec::<LongestSession>::new();
    let mut current_tracks = Vec::<HistoryEvent>::new();
    let mut current_start = events[0].played_at;
    let mut current_end = track_end(&events[0]);

    for event in events {
        let event_end = track_end(event);
        if !current_tracks.is_empty() && event.played_at > current_end + Duration::minutes(10) {
            sessions.push(build_session(
                current_start,
                current_end,
                std::mem::take(&mut current_tracks),
            ));
            current_start = event.played_at;
            current_end = event_end;
        } else if event_end > current_end {
            current_end = event_end;
        }
        current_tracks.push(event.clone());
    }

    if !current_tracks.is_empty() {
        sessions.push(build_session(current_start, current_end, current_tracks));
    }

    sessions
}

fn session_stats(sessions: &[LongestSession]) -> ListeningSessionStats {
    if sessions.is_empty() {
        return ListeningSessionStats {
            total_sessions: 0,
            average_duration_ms: 0,
            average_listens: 0.0,
            longest: None,
            most_intense: None,
        };
    }

    let total_duration = sessions
        .iter()
        .map(|session| session.duration_ms)
        .sum::<i64>();
    let total_listens = sessions.iter().map(|session| session.listens).sum::<i64>();
    let longest = sessions
        .iter()
        .max_by(|a, b| {
            a.duration_ms
                .cmp(&b.duration_ms)
                .then_with(|| b.start.cmp(&a.start))
        })
        .map(summarize_session);
    let intense_pool = sessions
        .iter()
        .filter(|session| session.listens >= 3)
        .collect::<Vec<_>>();
    let most_intense = if intense_pool.is_empty() {
        sessions.iter().max_by(compare_session_intensity)
    } else {
        intense_pool.into_iter().max_by(compare_session_intensity)
    }
    .map(summarize_session);

    ListeningSessionStats {
        total_sessions: sessions.len() as i64,
        average_duration_ms: total_duration / sessions.len() as i64,
        average_listens: total_listens as f64 / sessions.len() as f64,
        longest,
        most_intense,
    }
}

fn compare_session_intensity(a: &&LongestSession, b: &&LongestSession) -> std::cmp::Ordering {
    session_intensity(a)
        .partial_cmp(&session_intensity(b))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.listens.cmp(&b.listens))
        .then_with(|| a.duration_ms.cmp(&b.duration_ms))
}

fn session_intensity(session: &LongestSession) -> f64 {
    if session.duration_ms <= 0 {
        return 0.0;
    }
    session.listens as f64 / (session.duration_ms as f64 / 3_600_000.0)
}

fn summarize_session(session: &LongestSession) -> ListeningSessionSummary {
    let first_track = session.tracks.first();
    let last_track = session.tracks.last().or(first_track);
    let unique_artists = session
        .tracks
        .iter()
        .map(|track| track.artist_id.as_str())
        .collect::<HashSet<_>>()
        .len() as i64;

    ListeningSessionSummary {
        start: session.start,
        end: session.end,
        duration_ms: session.duration_ms,
        listens: session.listens,
        unique_artists,
        first_track_name: first_track
            .map(|track| track.track_name.clone())
            .unwrap_or_default(),
        last_track_name: last_track
            .map(|track| track.track_name.clone())
            .unwrap_or_default(),
        image_url: first_track.and_then(|track| track.image_url.clone()),
        listens_per_hour: session_intensity(session),
    }
}

#[derive(Clone)]
struct TrackLoopBuilder {
    track_id: String,
    track_name: String,
    artist_name: String,
    image_url: Option<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    last_played_at: DateTime<Utc>,
    listening_duration_ms: i64,
    listens: i64,
}

impl TrackLoopBuilder {
    fn new(event: &HistoryEvent) -> Self {
        Self {
            track_id: event.track_id.clone(),
            track_name: event.track_name.clone(),
            artist_name: event.artist_name.clone(),
            image_url: event.image_url.clone(),
            start: event.played_at,
            end: track_end(event),
            last_played_at: event.played_at,
            listening_duration_ms: event.duration_ms as i64,
            listens: 1,
        }
    }

    fn push(&mut self, event: &HistoryEvent) {
        self.end = self.end.max(track_end(event));
        self.last_played_at = event.played_at;
        self.listening_duration_ms += event.duration_ms as i64;
        self.listens += 1;
    }

    fn summary(&self) -> RepeatLoopSummary {
        RepeatLoopSummary {
            track_id: self.track_id.clone(),
            track_name: self.track_name.clone(),
            artist_name: self.artist_name.clone(),
            image_url: self.image_url.clone(),
            start: self.start,
            end: self.end,
            span_ms: (self.end - self.start).num_milliseconds(),
            listening_duration_ms: self.listening_duration_ms,
            listens: self.listens,
        }
    }
}

#[derive(Clone)]
struct ArtistRunBuilder {
    artist_id: String,
    artist_name: String,
    image_url: Option<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    listening_duration_ms: i64,
    listens: i64,
}

impl ArtistRunBuilder {
    fn new(event: &HistoryEvent) -> Self {
        Self {
            artist_id: event.artist_id.clone(),
            artist_name: event.artist_name.clone(),
            image_url: event.image_url.clone(),
            start: event.played_at,
            end: track_end(event),
            listening_duration_ms: event.duration_ms as i64,
            listens: 1,
        }
    }

    fn push(&mut self, event: &HistoryEvent) {
        self.end = self.end.max(track_end(event));
        self.listening_duration_ms += event.duration_ms as i64;
        self.listens += 1;
    }

    fn summary(&self) -> ArtistRunSummary {
        ArtistRunSummary {
            artist_id: self.artist_id.clone(),
            artist_name: self.artist_name.clone(),
            image_url: self.image_url.clone(),
            start: self.start,
            end: self.end,
            span_ms: (self.end - self.start).num_milliseconds(),
            listening_duration_ms: self.listening_duration_ms,
            listens: self.listens,
        }
    }
}

fn repeat_loop_stats(events: &[HistoryEvent]) -> RepeatLoopStats {
    let mut total_back_to_back_repeats = 0_i64;
    let mut top_track_loop: Option<RepeatLoopSummary> = None;
    let mut back_to_back_track_run: Option<RepeatLoopSummary> = None;
    let mut longest_artist_run: Option<ArtistRunSummary> = None;
    let mut active_track_clusters = HashMap::<String, TrackLoopBuilder>::new();
    let mut consecutive_track_run: Option<TrackLoopBuilder> = None;
    let mut consecutive_artist_run: Option<ArtistRunBuilder> = None;
    let mut previous_track_id: Option<&str> = None;

    for event in events {
        if previous_track_id == Some(event.track_id.as_str()) {
            total_back_to_back_repeats += 1;
        }
        previous_track_id = Some(event.track_id.as_str());

        let should_extend_cluster = active_track_clusters
            .get(&event.track_id)
            .map(|cluster| event.played_at <= cluster.last_played_at + Duration::minutes(30))
            .unwrap_or(false);
        if should_extend_cluster {
            if let Some(cluster) = active_track_clusters.get_mut(&event.track_id) {
                cluster.push(event);
            }
        } else {
            if let Some(cluster) = active_track_clusters.remove(&event.track_id) {
                update_track_loop(&mut top_track_loop, cluster.summary());
            }
            active_track_clusters.insert(event.track_id.clone(), TrackLoopBuilder::new(event));
        }

        if consecutive_track_run
            .as_ref()
            .map(|run| run.track_id == event.track_id)
            .unwrap_or(false)
        {
            if let Some(run) = consecutive_track_run.as_mut() {
                run.push(event);
            }
        } else {
            if let Some(run) = consecutive_track_run.take() {
                update_track_loop(&mut back_to_back_track_run, run.summary());
            }
            consecutive_track_run = Some(TrackLoopBuilder::new(event));
        }

        if consecutive_artist_run
            .as_ref()
            .map(|run| run.artist_id == event.artist_id)
            .unwrap_or(false)
        {
            if let Some(run) = consecutive_artist_run.as_mut() {
                run.push(event);
            }
        } else {
            if let Some(run) = consecutive_artist_run.take() {
                update_artist_run(&mut longest_artist_run, run.summary());
            }
            consecutive_artist_run = Some(ArtistRunBuilder::new(event));
        }
    }

    for cluster in active_track_clusters.into_values() {
        update_track_loop(&mut top_track_loop, cluster.summary());
    }
    if let Some(run) = consecutive_track_run {
        update_track_loop(&mut back_to_back_track_run, run.summary());
    }
    if let Some(run) = consecutive_artist_run {
        update_artist_run(&mut longest_artist_run, run.summary());
    }

    RepeatLoopStats {
        total_back_to_back_repeats,
        top_track_loop,
        back_to_back_track_run,
        longest_artist_run,
    }
}

fn update_track_loop(current: &mut Option<RepeatLoopSummary>, candidate: RepeatLoopSummary) {
    if candidate.listens < 2 {
        return;
    }
    if current
        .as_ref()
        .map(|existing| {
            candidate
                .listens
                .cmp(&existing.listens)
                .then_with(|| {
                    candidate
                        .listening_duration_ms
                        .cmp(&existing.listening_duration_ms)
                })
                .then_with(|| existing.start.cmp(&candidate.start))
                .is_gt()
        })
        .unwrap_or(true)
    {
        *current = Some(candidate);
    }
}

fn update_artist_run(current: &mut Option<ArtistRunSummary>, candidate: ArtistRunSummary) {
    if candidate.listens < 2 {
        return;
    }
    if current
        .as_ref()
        .map(|existing| {
            candidate
                .listens
                .cmp(&existing.listens)
                .then_with(|| {
                    candidate
                        .listening_duration_ms
                        .cmp(&existing.listening_duration_ms)
                })
                .then_with(|| existing.start.cmp(&candidate.start))
                .is_gt()
        })
        .unwrap_or(true)
    {
        *current = Some(candidate);
    }
}

fn track_end(event: &HistoryEvent) -> DateTime<Utc> {
    event.played_at + Duration::milliseconds(event.duration_ms as i64)
}

fn build_session(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    tracks: Vec<HistoryEvent>,
) -> LongestSession {
    LongestSession {
        start,
        end,
        duration_ms: (end - start).num_milliseconds(),
        listens: tracks.len() as i64,
        tracks,
    }
}

fn metric_order_expression(metric: Metric) -> &'static str {
    match metric {
        Metric::Count => "COUNT(*)",
        Metric::Duration => "COALESCE(SUM(le.duration_ms), 0)",
    }
}

fn bucket_grain(split: TimeSplit) -> &'static str {
    match split {
        TimeSplit::All => "day",
        TimeSplit::Year => "year",
        TimeSplit::Month => "month",
        TimeSplit::Week => "week",
        TimeSplit::Day => "day",
        TimeSplit::Hour => "hour",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn event(track_id: &str, minute: u32, duration_ms: i32) -> HistoryEvent {
        HistoryEvent {
            id: Uuid::new_v4(),
            track_id: track_id.to_owned(),
            track_name: track_id.to_owned(),
            album_id: "album".to_owned(),
            album_name: "Album".to_owned(),
            artist_id: "artist".to_owned(),
            artist_name: "Artist".to_owned(),
            image_url: None,
            duration_ms,
            played_at: Utc.with_ymd_and_hms(2024, 1, 1, 12, minute, 0).unwrap(),
            source: "seed".to_owned(),
        }
    }

    #[test]
    fn longest_sessions_split_after_ten_minute_gap_from_previous_end() {
        let events = vec![
            event("a", 0, 180_000),
            event("b", 5, 180_000),
            event("c", 30, 180_000),
        ];
        let sessions = build_longest_sessions(&events, 10);

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].listens, 2);
        assert_eq!(sessions[1].listens, 1);
    }

    #[test]
    fn longest_sessions_are_returned_by_duration_desc() {
        let events = vec![
            event("short", 0, 60_000),
            event("long-a", 20, 300_000),
            event("long-b", 25, 300_000),
        ];
        let sessions = build_longest_sessions(&events, 1);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tracks[0].track_id, "long-a");
    }
}
