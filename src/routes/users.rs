use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{delete, get, patch},
};

use crate::{
    auth::extractors::current_user,
    dto::{
        requests::ProfilePatchRequest,
        responses::{MeResponse, PublicSharingResponse, PublicTokenResponse},
    },
    error::{AppError, Result},
    repositories::{public_tokens, response_cache, settings, users},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/me", get(me))
        .route("/users/me/settings", patch(update_settings))
        .route("/users/me/profile", patch(update_profile))
        .route("/users/me/spotify-connection", delete(disconnect_spotify))
        .route(
            "/users/me/public-token",
            get(get_public_token)
                .post(create_public_token)
                .delete(delete_public_token),
        )
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    responses((status = 200, description = "Current user", body = MeResponse))
)]
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<MeResponse>> {
    let user = current_user(&headers, &state).await?;
    let user_settings = settings::get(&state.db, user.id).await?;
    let public_sharing_enabled = public_tokens::enabled_for_user(&state.db, user.id).await?;
    Ok(Json(MeResponse {
        user: user.into(),
        settings: user_settings,
        public_sharing: PublicSharingResponse {
            enabled: public_sharing_enabled,
            token: None,
        },
    }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/me/settings",
    request_body = crate::domain::settings::SettingsPatch,
    responses((status = 200, description = "Updated settings", body = MeResponse))
)]
pub async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<crate::domain::settings::SettingsPatch>,
) -> Result<Json<MeResponse>> {
    let user = current_user(&headers, &state).await?;
    let user_settings = settings::update(&state.db, user.id, &patch).await?;
    response_cache::invalidate_stats(&state.db, user.id).await?;
    let public_sharing_enabled = public_tokens::enabled_for_user(&state.db, user.id).await?;
    Ok(Json(MeResponse {
        user: user.into(),
        settings: user_settings,
        public_sharing: PublicSharingResponse {
            enabled: public_sharing_enabled,
            token: None,
        },
    }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/me/profile",
    request_body = ProfilePatchRequest,
    responses((status = 200, description = "Updated profile", body = MeResponse))
)]
pub async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<ProfilePatchRequest>,
) -> Result<Json<MeResponse>> {
    let user = current_user(&headers, &state).await?;
    if let Some(username) = &patch.username
        && username.trim().is_empty()
    {
        return Err(AppError::validation("username must not be empty"));
    }
    let updated = users::update_profile(&state.db, user.id, patch.username.as_deref()).await?;
    let user_settings = settings::get(&state.db, user.id).await?;
    let public_sharing_enabled = public_tokens::enabled_for_user(&state.db, updated.id).await?;
    Ok(Json(MeResponse {
        user: updated.into(),
        settings: user_settings,
        public_sharing: PublicSharingResponse {
            enabled: public_sharing_enabled,
            token: None,
        },
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/me/spotify-connection",
    responses((status = 204, description = "Deleted stored Spotify tokens and stopped polling"))
)]
pub async fn disconnect_spotify(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    users::clear_spotify_tokens(&state.db, user.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me/public-token",
    responses((status = 200, description = "Current public token state", body = PublicSharingResponse))
)]
pub async fn get_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PublicSharingResponse>> {
    let user = current_user(&headers, &state).await?;
    let enabled = public_tokens::enabled_for_user(&state.db, user.id).await?;
    Ok(Json(PublicSharingResponse {
        enabled,
        token: None,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/me/public-token",
    responses((status = 200, description = "Created public token", body = PublicTokenResponse))
)]
pub async fn create_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PublicTokenResponse>> {
    let user = current_user(&headers, &state).await?;
    let token = public_tokens::generate_token();
    public_tokens::rotate(&state.db, user.id, &token).await?;
    Ok(Json(PublicTokenResponse { token }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/me/public-token",
    responses((status = 204, description = "Deleted public token"))
)]
pub async fn delete_public_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    public_tokens::delete_for_user(&state.db, user.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
