use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::user::User, error::Result};

pub const SESSION_COOKIE: &str = "ys_session";

pub fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    raw_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    let token_hash = hash_token(raw_token);
    sqlx::query(
        r#"
        INSERT INTO sessions (token_hash, user_id, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn user_for_token(pool: &PgPool, raw_token: &str) -> Result<Option<User>> {
    let token_hash = hash_token(raw_token);
    let mut tx = pool.begin().await?;
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.id, u.username, u.spotify_id, u.admin, u.access_token, u.refresh_token,
               u.token_expires_at, u.last_spotify_poll_at, u.first_listened_at, u.created_at, u.updated_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.expires_at > now()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    if user.is_some() {
        sqlx::query("UPDATE sessions SET last_seen_at = now() WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(user)
}

pub async fn delete(pool: &PgPool, raw_token: &str) -> Result<bool> {
    let token_hash = hash_token(raw_token);
    let result = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn cleanup_expired(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_hashes_are_deterministic_and_not_raw() {
        let token = "raw-token";
        assert_eq!(hash_token(token), hash_token(token));
        assert_ne!(hash_token(token), token);
    }
}
