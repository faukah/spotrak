use crate::db::{PgPool, Transaction};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::user::{NewUser, PublicUser, User},
    error::Result,
};

pub async fn count_tx(tx: &Transaction<'_>) -> Result<i64> {
    let count = crate::db::query_scalar::<i64>("SELECT COUNT(*) FROM users")
        .fetch_one(tx)
        .await?;
    Ok(count)
}

#[allow(dead_code)]
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    let user = crate::db::query_as::<User>(
        r#"
        SELECT id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
               last_spotify_poll_at, first_listened_at, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_by_spotify_id_tx(tx: &Transaction<'_>, spotify_id: &str) -> Result<Option<User>> {
    let user = crate::db::query_as::<User>(
        r#"
        SELECT id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
               last_spotify_poll_at, first_listened_at, created_at, updated_at
        FROM users
        WHERE spotify_id = $1
        "#,
    )
    .bind(spotify_id)
    .fetch_optional(tx)
    .await?;
    Ok(user)
}

pub async fn upsert_login(tx: &Transaction<'_>, new_user: &NewUser) -> Result<User> {
    let user = crate::db::query_as::<User>(
        r#"
        INSERT INTO users (username, spotify_id, admin)
        VALUES ($1, $2, $3)
        ON CONFLICT (spotify_id) DO UPDATE SET
          username = EXCLUDED.username,
          updated_at = now()
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(&new_user.username)
    .bind(&new_user.spotify_id)
    .bind(new_user.admin)
    .fetch_one(tx)
    .await?;
    Ok(user)
}

#[allow(dead_code)]
pub async fn update_tokens(
    pool: &PgPool,
    user_id: Uuid,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: DateTime<Utc>,
) -> Result<User> {
    let user = crate::db::query_as::<User>(
        r#"
        UPDATE users
        SET access_token = $2,
            refresh_token = COALESCE($3, refresh_token),
            token_expires_at = $4,
            updated_at = now()
        WHERE id = $1
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(token_expires_at)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn update_tokens_tx(
    tx: &Transaction<'_>,
    user_id: Uuid,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: DateTime<Utc>,
) -> Result<User> {
    let user = crate::db::query_as::<User>(
        r#"
        UPDATE users
        SET access_token = $2,
            refresh_token = COALESCE($3, refresh_token),
            token_expires_at = $4,
            updated_at = now()
        WHERE id = $1
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(token_expires_at)
    .fetch_one(tx)
    .await?;
    Ok(user)
}

pub async fn update_tokens_if_refresh_token_matches(
    pool: &PgPool,
    user_id: Uuid,
    access_token: &str,
    refresh_token: Option<&str>,
    token_expires_at: DateTime<Utc>,
    expected_refresh_token: Option<&str>,
) -> Result<Option<User>> {
    let user = crate::db::query_as::<User>(
        r#"
        UPDATE users
        SET access_token = $2,
            refresh_token = COALESCE($3, refresh_token),
            token_expires_at = $4,
            updated_at = now()
        WHERE id = $1
          AND refresh_token IS NOT DISTINCT FROM $5
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(token_expires_at)
    .bind(expected_refresh_token)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn clear_spotify_tokens(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let result = crate::db::query(
        r#"
        UPDATE users
        SET access_token = NULL,
            refresh_token = NULL,
            token_expires_at = NULL,
            last_spotify_poll_at = NULL,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_profile(pool: &PgPool, user_id: Uuid, username: Option<&str>) -> Result<User> {
    let user = crate::db::query_as::<User>(
        r#"
        UPDATE users
        SET username = COALESCE($2, username), updated_at = now()
        WHERE id = $1
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(username)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn list(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<PublicUser>> {
    let users = crate::db::query_as::<PublicUser>(
        r#"
        SELECT id, username, spotify_id, admin, created_at
        FROM users
        ORDER BY created_at ASC, id ASC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(users)
}

pub async fn set_admin(pool: &PgPool, user_id: Uuid, admin: bool) -> Result<User> {
    let user = crate::db::query_as::<User>(
        r#"
        UPDATE users
        SET admin = $2, updated_at = now()
        WHERE id = $1
        RETURNING id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
                  last_spotify_poll_at, first_listened_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(admin)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn delete(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let result = crate::db::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_pollable(pool: &PgPool) -> Result<Vec<User>> {
    let users = crate::db::query_as::<User>(
        r#"
        SELECT id, username, spotify_id, admin, access_token, refresh_token, token_expires_at,
               last_spotify_poll_at, first_listened_at, created_at, updated_at
        FROM users
        WHERE refresh_token IS NOT NULL
        ORDER BY last_spotify_poll_at ASC NULLS FIRST, created_at ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(users)
}

pub async fn update_poll_markers(
    pool: &PgPool,
    user_id: Uuid,
    last_poll_at: DateTime<Utc>,
    first_seen_at: Option<DateTime<Utc>>,
) -> Result<()> {
    crate::db::query(
        r#"
        UPDATE users
        SET last_spotify_poll_at = $2,
            first_listened_at = CASE
              WHEN $3::timestamptz IS NULL THEN first_listened_at
              WHEN first_listened_at IS NULL THEN $3
              WHEN first_listened_at > $3 THEN $3
              ELSE first_listened_at
            END,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(last_poll_at)
    .bind(first_seen_at)
    .execute(pool)
    .await?;
    Ok(())
}
