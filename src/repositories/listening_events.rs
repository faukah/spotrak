use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        stats::{
            AlbumReleaseYearPoint, AlbumReleaseYearsStats, BucketedTopAlbum, BucketedTopArtist,
            BucketedTopTrack, DiversityTimelinePoint, EntityStats, FeatureRatioStats, HistoryEvent,
            HourRepartitionPoint, LongestSession, SummaryStats, TimelinePoint, TopAlbum, TopArtist,
            TopTrack,
        },
        time::{Metric, TimeSplit},
    },
    error::Result,
};

pub async fn history(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEvent>> {
    let rows = sqlx::query_as::<_, HistoryEvent>(
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
    let stats = sqlx::query_as::<_, SummaryStats>(
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
        let rows = sqlx::query_as::<_, TimelinePoint>(
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
        let row = sqlx::query_as::<_, TimelinePoint>(
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
    let rows = sqlx::query_as::<_, TopTrack>(&sql)
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
                 (array_remove(array_agg(COALESCE(t.images->0->>'url', al.images->0->>'url') ORDER BY le.played_at DESC), NULL))[1]
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
    let rows = sqlx::query_as::<_, TopArtist>(&sql)
        .bind(user_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
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
        LEFT JOIN album_artists aa ON aa.album_id = a.id AND aa.position = 0
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
    let rows = sqlx::query_as::<_, TopAlbum>(&sql)
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
    let rows = sqlx::query_as::<_, HourRepartitionPoint>(
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
    let stats = sqlx::query_as::<_, FeatureRatioStats>(
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

pub async fn album_release_years(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<AlbumReleaseYearsStats> {
    let distribution = sqlx::query_as::<_, AlbumReleaseYearPoint>(
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
    let rows = sqlx::query_as::<_, DiversityTimelinePoint>(
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
    let stats = sqlx::query_as::<_, EntityStats>(&sql)
        .bind(user_id)
        .bind(id)
        .bind(start)
        .bind(end)
        .fetch_one(pool)
        .await?;
    Ok(stats)
}

pub async fn delete_imported_history(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM listening_events
        WHERE user_id = $1
          AND source IN ('privacy-import', 'full-privacy-import')
        "#,
    )
    .bind(user_id)
    .execute(&mut **tx)
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
    Ok(build_longest_sessions(events, limit))
}

async fn history_ascending(
    pool: &PgPool,
    user_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<HistoryEvent>> {
    let rows = sqlx::query_as::<_, HistoryEvent>(
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

fn build_longest_sessions(events: Vec<HistoryEvent>, limit: i64) -> Vec<LongestSession> {
    if events.is_empty() {
        return Vec::new();
    }

    let mut sessions = Vec::<LongestSession>::new();
    let mut current_tracks = Vec::<HistoryEvent>::new();
    let mut current_start = events[0].played_at;
    let mut current_end = track_end(&events[0]);

    for event in events {
        let event_end = track_end(&event);
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
        current_tracks.push(event);
    }

    if !current_tracks.is_empty() {
        sessions.push(build_session(current_start, current_end, current_tracks));
    }

    sessions.sort_by(|a, b| {
        b.duration_ms
            .cmp(&a.duration_ms)
            .then_with(|| a.start.cmp(&b.start))
            .then_with(|| a.end.cmp(&b.end))
    });
    sessions.truncate(limit as usize);
    sessions
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
        let sessions = build_longest_sessions(
            vec![
                event("a", 0, 180_000),
                event("b", 5, 180_000),
                event("c", 30, 180_000),
            ],
            10,
        );

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].listens, 2);
        assert_eq!(sessions[1].listens, 1);
    }

    #[test]
    fn longest_sessions_are_returned_by_duration_desc() {
        let sessions = build_longest_sessions(
            vec![
                event("short", 0, 60_000),
                event("long-a", 20, 300_000),
                event("long-b", 25, 300_000),
            ],
            1,
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tracks[0].track_id, "long-a");
    }
}

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
                   (array_remove(array_agg(COALESCE(t.images->0->>'url', a.images->0->>'url') ORDER BY le.played_at DESC), NULL))[1]
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
          LEFT JOIN album_artists aa ON aa.album_id = a.id AND aa.position = 0
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
