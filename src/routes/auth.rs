use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    auth::{
        extractors::current_user,
        sessions::{login_cookie, logout_cookie, session_token_from_headers},
    },
    dto::responses::MeResponse,
    error::{AppError, Result},
    repositories::{public_tokens, settings},
    services::{auth as auth_service, spotify_client},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/spotify/start", get(spotify_start))
        .route("/auth/spotify/callback", get(spotify_callback))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/spotify/profile", get(spotify_profile))
}

#[derive(Debug, Deserialize)]
pub struct SpotifyCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/spotify/start",
    responses((status = 307, description = "Redirects to Spotify OAuth"))
)]
pub async fn spotify_start(State(state): State<AppState>) -> Result<Redirect> {
    let url = auth_service::start_spotify_login(&state).await?;
    Ok(Redirect::temporary(&url))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/spotify/callback",
    params(("code" = Option<String>, Query), ("state" = Option<String>, Query), ("error" = Option<String>, Query)),
    responses((status = 303, description = "Sets session cookie and redirects to frontend"))
)]
pub async fn spotify_callback(
    State(state): State<AppState>,
    Query(query): Query<SpotifyCallbackQuery>,
) -> Result<impl IntoResponse> {
    if let Some(error) = query.error {
        return Err(AppError::spotify(error));
    }
    let code = query
        .code
        .ok_or_else(|| AppError::validation("missing OAuth code"))?;
    let oauth_state = query
        .state
        .ok_or_else(|| AppError::validation("missing OAuth state"))?;
    let login = auth_service::complete_spotify_login(&state, &oauth_state, &code).await?;

    let mut response = Redirect::to(state.config.client_endpoint.as_str()).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        login_cookie(&state.config, &login.session_token)
            .parse()
            .map_err(|err| AppError::internal(format!("invalid Set-Cookie header: {err}")))?,
    );
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses((status = 204, description = "Session deleted"))
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let token = session_token_from_headers(&headers);
    auth_service::logout(&state, token.as_deref()).await?;
    let mut response = axum::http::StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        logout_cookie(&state.config)
            .parse()
            .map_err(|err| AppError::internal(format!("invalid Set-Cookie header: {err}")))?,
    );
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses((status = 200, description = "Current user", body = MeResponse), (status = 401, description = "Unauthorized"))
)]
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<MeResponse>> {
    let user = current_user(&headers, &state).await?;
    let user_settings = settings::get(&state.db, user.id).await?;
    let public_token = public_tokens::token_for_user(&state.db, user.id).await?;
    Ok(Json(MeResponse {
        user: user.into(),
        settings: user_settings,
        public_sharing: crate::dto::responses::PublicSharingResponse {
            enabled: public_token.is_some(),
            token: public_token,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/spotify/profile",
    responses((status = 200, description = "Current Spotify profile"), (status = 401, description = "Unauthorized"))
)]
pub async fn spotify_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::domain::spotify::SpotifyProfile>> {
    let user = current_user(&headers, &state).await?;
    let access_token = user.access_token.ok_or(AppError::Unauthorized)?;
    let profile = spotify_client::me(&state, &access_token).await?;
    Ok(Json(profile))
}
