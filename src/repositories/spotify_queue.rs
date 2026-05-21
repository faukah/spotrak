use chrono::{Duration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone, FromRow)]
pub struct ArtistHydrationJob {
    pub artist_id: String,
    pub user_id: Uuid,
    pub attempts: i32,
}

pub async fn enqueue_artist_hydration(
    pool: &PgPool,
    user_id: Uuid,
    artist_ids: &[String],
) -> Result<u64> {
    if artist_ids.is_empty() {
        return Ok(0);
    }

    let result = sqlx::query(
        r#"
        INSERT INTO spotify_artist_hydration_queue (artist_id, user_id)
        SELECT DISTINCT trim(id), $2
        FROM unnest($1::text[]) AS input(id)
        WHERE trim(id) <> ''
        ON CONFLICT (artist_id, user_id) DO UPDATE SET
          next_attempt_at = LEAST(spotify_artist_hydration_queue.next_attempt_at, now()),
          updated_at = now()
        "#,
    )
    .bind(artist_ids)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn claim_artist_hydration_jobs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ArtistHydrationJob>> {
    let rows = sqlx::query_as::<_, ArtistHydrationJob>(
        r#"
        WITH due AS (
          SELECT artist_id, user_id
          FROM spotify_artist_hydration_queue
          WHERE next_attempt_at <= now()
          ORDER BY created_at ASC, artist_id ASC, user_id ASC
          LIMIT $1
          FOR UPDATE SKIP LOCKED
        )
        UPDATE spotify_artist_hydration_queue queue
        SET attempts = queue.attempts + 1,
            next_attempt_at = now() + interval '15 minutes',
            updated_at = now()
        FROM due
        WHERE queue.artist_id = due.artist_id
          AND queue.user_id = due.user_id
        RETURNING queue.artist_id, queue.user_id, queue.attempts
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn complete_artist_hydration(pool: &PgPool, artist_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM spotify_artist_hydration_queue WHERE artist_id = $1")
        .bind(artist_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn fail_artist_hydration(
    pool: &PgPool,
    artist_id: &str,
    user_id: Uuid,
    attempts: i32,
    error: &str,
) -> Result<()> {
    let capped_attempts = attempts.clamp(1, 8);
    let backoff_minutes = 2_i64.pow(capped_attempts as u32).min(240);
    let next_attempt_at = Utc::now() + Duration::minutes(backoff_minutes);

    sqlx::query(
        r#"
        UPDATE spotify_artist_hydration_queue
        SET next_attempt_at = $2,
            last_error = left($3, 1000),
            updated_at = now()
        WHERE artist_id = $1 AND user_id = $4
        "#,
    )
    .bind(artist_id)
    .bind(next_attempt_at)
    .bind(error)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
