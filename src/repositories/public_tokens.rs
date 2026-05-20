use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::Result, repositories::sessions};

pub fn generate_token() -> String {
    sessions::generate_token()
}

pub fn hash_token(token: &str) -> String {
    sessions::hash_token(token)
}

pub async fn rotate(pool: &PgPool, user_id: Uuid, raw_token: &str) -> Result<()> {
    let token_hash = hash_token(raw_token);
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM public_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO public_tokens (token_hash, user_id, token_value)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(raw_token)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn token_for_user(pool: &PgPool, user_id: Uuid) -> Result<Option<String>> {
    let token = sqlx::query_scalar::<_, String>(
        "SELECT token_value FROM public_tokens WHERE user_id = $1 AND token_value IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(token)
}

pub async fn delete_for_user(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM public_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn user_id_for_token(pool: &PgPool, raw_token: &str) -> Result<Option<Uuid>> {
    let token_hash = hash_token(raw_token);
    let user_id =
        sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM public_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_optional(pool)
            .await?;
    Ok(user_id)
}
