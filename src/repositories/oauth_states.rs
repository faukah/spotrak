use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::Result;

pub fn generate_state() -> String {
    random_url_safe_secret()
}

pub fn generate_code_verifier() -> String {
    random_url_safe_secret()
}

pub fn code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn random_url_safe_secret() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_state(state: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(state.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub async fn create(
    pool: &PgPool,
    raw_state: &str,
    code_verifier: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    let state_hash = hash_state(raw_state);
    sqlx::query(
        r#"
        INSERT INTO oauth_states (state_hash, code_verifier, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(state_hash)
    .bind(code_verifier)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn consume(pool: &PgPool, raw_state: &str) -> Result<Option<String>> {
    let state_hash = hash_state(raw_state);
    let code_verifier = sqlx::query_scalar::<_, Option<String>>(
        r#"
        DELETE FROM oauth_states
        WHERE state_hash = $1 AND expires_at > now()
        RETURNING code_verifier
        "#,
    )
    .bind(state_hash)
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(code_verifier)
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

    #[test]
    fn code_challenge_is_deterministic_and_not_raw() {
        let verifier = "code-verifier";
        assert_eq!(code_challenge(verifier), code_challenge(verifier));
        assert_ne!(code_challenge(verifier), verifier);
    }
}
