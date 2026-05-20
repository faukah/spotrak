use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::Result, repositories::sessions};

const PUBLIC_TOKEN_VALIDITY_DAYS: i64 = 365;

pub fn generate_token() -> String {
    sessions::generate_token()
}

pub fn hash_token(token: &str) -> String {
    sessions::hash_token(token)
}

pub async fn rotate(pool: &PgPool, user_id: Uuid, raw_token: &str) -> Result<()> {
    let token_hash = hash_token(raw_token);
    let expires_at = Utc::now() + Duration::days(PUBLIC_TOKEN_VALIDITY_DAYS);
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE public_tokens
        SET revoked_at = now()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO public_tokens (token_hash, user_id, expires_at, rotated_at)
        VALUES ($1, $2, $3, now())
        "#,
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn enabled_for_user(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM public_tokens
          WHERE user_id = $1
            AND revoked_at IS NULL
            AND expires_at > now()
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(enabled)
}

pub async fn delete_for_user(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE public_tokens
        SET revoked_at = now()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn user_id_for_token(pool: &PgPool, raw_token: &str) -> Result<Option<Uuid>> {
    let token_hash = hash_token(raw_token);
    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT user_id
        FROM public_tokens
        WHERE token_hash = $1
          AND revoked_at IS NULL
          AND expires_at > now()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(user_id)
}
