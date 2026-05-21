use chrono::{Duration as ChronoDuration, Utc};
use reqwest::StatusCode;
use serde::Serialize;

use crate::{
    domain::user::User,
    error::{AppError, Result},
    repositories::users,
    services::{ingestion, spotify_client, token_crypto},
    state::AppState,
};

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PollSummary {
    pub users_seen: usize,
    pub users_polled: usize,
    pub events_inserted: usize,
    pub errors: usize,
}

pub async fn poll_once(state: &AppState) -> Result<PollSummary> {
    let pollable_users = users::list_pollable(&state.db).await?;
    let mut summary = PollSummary {
        users_seen: pollable_users.len(),
        ..PollSummary::default()
    };

    for user in pollable_users {
        match poll_user(state, user).await {
            Ok(inserted) => {
                summary.users_polled += 1;
                summary.events_inserted += inserted;
            }
            Err(error) => {
                summary.errors += 1;
                tracing::warn!(?error, "failed to poll Spotify for user");
            }
        }
    }

    Ok(summary)
}

async fn poll_user(state: &AppState, user: User) -> Result<usize> {
    let mut access_token = valid_access_token(state, &user).await?;
    let after = user
        .last_spotify_poll_at
        .map(|last_poll| last_poll - ChronoDuration::hours(2));
    let items = match spotify_client::recently_played_after(state, &access_token, after).await {
        Ok(items) => items,
        Err(error) if is_spotify_authorization_error(&error) => {
            tracing::info!(
                user_id = %user.id,
                "refreshing Spotify token after recently-played authorization failure"
            );
            access_token = force_refresh_access_token(state, user.id).await?;
            spotify_client::recently_played_after(state, &access_token, after).await?
        }
        Err(error) => return Err(error),
    };
    let result = ingestion::ingest_recently_played(state, user.id, &items).await?;
    users::update_poll_markers(&state.db, user.id, Utc::now(), result.earliest_played_at).await?;

    Ok(result.inserted)
}

pub async fn valid_access_token(state: &AppState, user: &User) -> Result<String> {
    let refresh_at = Utc::now() + ChronoDuration::minutes(2);
    if let (Some(access_token), Some(expires_at)) = (&user.access_token, user.token_expires_at) {
        if expires_at > refresh_at {
            return token_crypto::decrypt_spotify_token(&state.config, access_token);
        }
    }

    refresh_access_token(state, user).await
}

pub async fn refresh_access_token(state: &AppState, user: &User) -> Result<String> {
    refresh_access_token_for_user_id(state, user.id, false).await
}

pub async fn force_refresh_access_token(state: &AppState, user_id: uuid::Uuid) -> Result<String> {
    refresh_access_token_for_user_id(state, user_id, true).await
}

async fn refresh_access_token_for_user_id(
    state: &AppState,
    user_id: uuid::Uuid,
    mut force: bool,
) -> Result<String> {
    let refresh_at = Utc::now() + ChronoDuration::minutes(2);

    for _ in 0..2 {
        let current = users::find_by_id(&state.db, user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if !force {
            if let (Some(access_token), Some(expires_at)) =
                (&current.access_token, current.token_expires_at)
            {
                if expires_at > refresh_at {
                    return token_crypto::decrypt_spotify_token(&state.config, access_token);
                }
            }
        }

        let original_refresh_token = current.refresh_token.clone();
        let refresh_token = original_refresh_token
            .as_deref()
            .ok_or(AppError::Unauthorized)
            .and_then(|token| token_crypto::decrypt_spotify_token(&state.config, token))?;
        let refreshed = spotify_client::refresh_token(state, &refresh_token).await?;
        let encrypted_access_token =
            token_crypto::encrypt_spotify_token(&state.config, &refreshed.access_token)?;
        let encrypted_refresh_token = refreshed
            .refresh_token
            .as_deref()
            .map(|token| token_crypto::encrypt_spotify_token(&state.config, token))
            .transpose()?;

        if users::update_tokens_if_refresh_token_matches(
            &state.db,
            current.id,
            &encrypted_access_token,
            encrypted_refresh_token.as_deref(),
            refreshed.token_expires_at,
            original_refresh_token.as_deref(),
        )
        .await?
        .is_some()
        {
            return Ok(refreshed.access_token);
        }

        tracing::debug!(
            user_id = %current.id,
            "Spotify token refresh raced with another worker; reloading current token"
        );
        force = false;
    }

    Err(AppError::Unauthorized)
}

pub fn is_spotify_authorization_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::SpotifyApi { status, .. } if *status == StatusCode::UNAUTHORIZED
    )
}
