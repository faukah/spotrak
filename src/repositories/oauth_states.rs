use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::Result;

pub fn generate_state() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_state(state: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(state.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub async fn create(pool: &PgPool, raw_state: &str, expires_at: DateTime<Utc>) -> Result<()> {
    let state_hash = hash_state(raw_state);
    sqlx::query(
        r#"
        INSERT INTO oauth_states (state_hash, expires_at)
        VALUES ($1, $2)
        "#,
    )
    .bind(state_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn consume(pool: &PgPool, raw_state: &str) -> Result<bool> {
    let state_hash = hash_state(raw_state);
    let result = sqlx::query(
        r#"
        DELETE FROM oauth_states
        WHERE state_hash = $1 AND expires_at > now()
        "#,
    )
    .bind(state_hash)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn cleanup_expired(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM oauth_states WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_state_hashes_are_deterministic_and_not_raw() {
        let state = "raw-state";
        assert_eq!(hash_state(state), hash_state(state));
        assert_ne!(hash_state(state), state);
    }
}
