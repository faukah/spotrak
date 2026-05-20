use chrono::{Duration as ChronoDuration, Utc};
use serde::Serialize;

use crate::{
    domain::user::User,
    error::{AppError, Result},
    repositories::users,
    services::{ingestion, spotify_client},
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
    let access_token = valid_access_token(state, &user).await?;
    let after = user
        .last_spotify_poll_at
        .map(|last_poll| last_poll - ChronoDuration::hours(2));
    let items = spotify_client::recently_played_after(state, &access_token, after).await?;
    let result = ingestion::ingest_recently_played(state, user.id, &items).await?;
    users::update_poll_markers(&state.db, user.id, Utc::now(), result.earliest_played_at).await?;

    Ok(result.inserted)
}

pub async fn valid_access_token(state: &AppState, user: &User) -> Result<String> {
    let refresh_at = Utc::now() + ChronoDuration::minutes(2);
    if let (Some(access_token), Some(expires_at)) = (&user.access_token, user.token_expires_at) {
        if expires_at > refresh_at {
            return Ok(access_token.clone());
        }
    }

    refresh_access_token(state, user).await
}

pub async fn refresh_access_token(state: &AppState, user: &User) -> Result<String> {
    let refresh_token = user
        .refresh_token
        .as_deref()
        .ok_or(AppError::Unauthorized)?;
    let refreshed = spotify_client::refresh_token(state, refresh_token).await?;
    users::update_tokens(
        &state.db,
        user.id,
        &refreshed.access_token,
        refreshed.refresh_token.as_deref(),
        refreshed.token_expires_at,
    )
    .await?;
    Ok(refreshed.access_token)
}
