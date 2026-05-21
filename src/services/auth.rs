use chrono::{Duration as ChronoDuration, Utc};

use crate::{
    domain::user::NewUser,
    error::{AppError, Result},
    repositories::{oauth_states, sessions, settings, users},
    services::{spotify_client, token_crypto},
    state::AppState,
};

pub struct LoginResult {
    pub session_token: String,
}

pub struct StartLoginResult {
    pub url: String,
    pub state: String,
}

pub async fn start_spotify_login(state: &AppState) -> Result<StartLoginResult> {
    let raw_state = oauth_states::generate_state();
    let code_verifier = oauth_states::generate_code_verifier();
    let code_challenge = oauth_states::code_challenge(&code_verifier);
    let expires_at = Utc::now() + ChronoDuration::minutes(10);
    oauth_states::create(&state.db, &raw_state, &code_verifier, expires_at).await?;
    Ok(StartLoginResult {
        url: spotify_client::authorize_url(&state.config, &raw_state, &code_challenge)?.to_string(),
        state: raw_state,
    })
}

pub async fn complete_spotify_login(
    state: &AppState,
    raw_state: &str,
    code: &str,
) -> Result<LoginResult> {
    let Some(code_verifier) = oauth_states::consume(&state.db, raw_state).await? else {
        return Err(AppError::validation("invalid or expired OAuth state"));
    };

    let tokens = spotify_client::exchange_code(state, code, &code_verifier).await?;
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

    let encrypted_access_token =
        token_crypto::encrypt_spotify_token(&state.config, &tokens.access_token)?;
    let encrypted_refresh_token = tokens
        .refresh_token
        .as_deref()
        .map(|token| token_crypto::encrypt_spotify_token(&state.config, token))
        .transpose()?;

    let mut tx = state.db.begin().await?;
    let new_user = NewUser {
        username,
        spotify_id: profile.id,
        admin: existing.map(|user| user.admin).unwrap_or(user_count == 0),
    };
    let user = users::upsert_login(&mut tx, &new_user).await?;
    let user = users::update_tokens_tx(
        &mut tx,
        user.id,
        &encrypted_access_token,
        encrypted_refresh_token.as_deref(),
        tokens.token_expires_at,
    )
    .await?;
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
