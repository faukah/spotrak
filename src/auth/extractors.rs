use axum::http::HeaderMap;

use crate::{
    auth::sessions::session_token_from_headers,
    domain::user::User,
    error::{AppError, Result},
    repositories::sessions,
    state::AppState,
};

pub async fn current_user(headers: &HeaderMap, state: &AppState) -> Result<User> {
    let token = session_token_from_headers(headers).ok_or(AppError::Unauthorized)?;
    sessions::user_for_token(&state.db, &token)
        .await?
        .ok_or(AppError::Unauthorized)
}

pub async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<User> {
    let user = current_user(headers, state).await?;
    if user.admin {
        Ok(user)
    } else {
        Err(AppError::Forbidden)
    }
}
