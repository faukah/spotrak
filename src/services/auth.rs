use chrono::{Duration as ChronoDuration, Utc};

use crate::{
    domain::user::NewUser,
    error::{AppError, Result},
    repositories::{oauth_states, sessions, settings, users},
    services::spotify_client,
    state::AppState,
};

pub struct LoginResult {
    pub session_token: String,
}

pub async fn start_spotify_login(state: &AppState) -> Result<String> {
    let raw_state = oauth_states::generate_state();
    let expires_at = Utc::now() + ChronoDuration::minutes(10);
    oauth_states::create(&state.db, &raw_state, expires_at).await?;
    Ok(spotify_client::authorize_url(&state.config, &raw_state)?.to_string())
}

pub async fn complete_spotify_login(
    state: &AppState,
    raw_state: &str,
    code: &str,
) -> Result<LoginResult> {
    if !oauth_states::consume(&state.db, raw_state).await? {
        return Err(AppError::validation("invalid or expired OAuth state"));
    }

    let tokens = spotify_client::exchange_code(state, code).await?;
    let profile = spotify_client::me(state, &tokens.access_token).await?;
    let username = profile
        .display_name
        .clone()
        .unwrap_or_else(|| profile.id.clone());

    let existing = users::find_by_spotify_id(&state.db, &profile.id).await?;
    let preferences = settings::global(&state.db).await?;
    let user_count = users::count(&state.db).await?;

    if existing.is_none() && !preferences.allow_registrations && user_count > 0 {
        return Err(AppError::Forbidden);
    }

    let mut tx = state.db.begin().await?;
    let user = if let Some(user) = existing {
        users::update_tokens_tx(
            &mut tx,
            user.id,
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
            tokens.token_expires_at,
        )
        .await?
    } else {
        let new_user = NewUser {
            username,
            spotify_id: profile.id,
            admin: user_count == 0,
        };
        let user = users::create(&mut tx, &new_user).await?;
        users::update_tokens_tx(
            &mut tx,
            user.id,
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
            tokens.token_expires_at,
        )
        .await?
    };
    settings::ensure_default(&mut tx, user.id).await?;
    tx.commit().await?;

    let session_token = sessions::generate_token();
    let expires_at = Utc::now()
        + ChronoDuration::from_std(state.config.cookie_validity)
            .map_err(|err| AppError::internal(err.to_string()))?;
    sessions::create(&state.db, user.id, &session_token, expires_at).await?;

    Ok(LoginResult { session_token })
}

pub async fn logout(state: &AppState, session_token: Option<&str>) -> Result<()> {
    if let Some(token) = session_token {
        sessions::delete(&state.db, token).await?;
    }
    Ok(())
}
