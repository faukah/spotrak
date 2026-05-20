use chrono::{DateTime, Duration, Utc};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, error::Result};

pub const STATS_OVERVIEW_NAMESPACE: &str = "stats_overview_v1";

pub async fn get<T>(
    pool: &PgPool,
    namespace: &str,
    user_id: Uuid,
    cache_key: &str,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let payload = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT payload
        FROM response_cache
        WHERE namespace = $1
          AND user_id = $2
          AND cache_key = $3
          AND (expires_at IS NULL OR expires_at > now())
        "#,
    )
    .bind(namespace)
    .bind(user_id)
    .bind(cache_key)
    .fetch_optional(pool)
    .await?;

    payload
        .map(|value| {
            serde_json::from_value(value).map_err(|err| AppError::internal(err.to_string()))
        })
        .transpose()
}

pub async fn set<T>(
    pool: &PgPool,
    namespace: &str,
    user_id: Uuid,
    cache_key: &str,
    value: &T,
    ttl: Option<Duration>,
) -> Result<()>
where
    T: Serialize,
{
    let payload = serde_json::to_value(value).map_err(|err| AppError::internal(err.to_string()))?;
    let expires_at: Option<DateTime<Utc>> = ttl.map(|ttl| Utc::now() + ttl);

    sqlx::query(
        r#"
        INSERT INTO response_cache (namespace, user_id, cache_key, payload, computed_at, expires_at)
        VALUES ($1, $2, $3, $4, now(), $5)
        ON CONFLICT (namespace, user_id, cache_key) DO UPDATE SET
          payload = EXCLUDED.payload,
          computed_at = EXCLUDED.computed_at,
          expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(namespace)
    .bind(user_id)
    .bind(cache_key)
    .bind(payload)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn invalidate_namespace(pool: &PgPool, namespace: &str, user_id: Uuid) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM response_cache
        WHERE namespace = $1 AND user_id = $2
        "#,
    )
    .bind(namespace)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn cleanup_expired(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM response_cache
        WHERE expires_at IS NOT NULL AND expires_at <= now()
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
