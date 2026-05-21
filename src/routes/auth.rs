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
        sessions::{
            clear_oauth_state_cookie, login_cookie, logout_cookie, oauth_state_cookie,
            oauth_state_from_headers, session_token_from_headers,
        },
    },
    dto::responses::MeResponse,
    error::{AppError, Result},
    repositories::{public_tokens, settings},
    services::auth as auth_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/spotify/start", get(spotify_start))
        .route("/auth/spotify/callback", get(spotify_callback))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

#[derive(Debug, Deserialize)]
pub struct SpotifyCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpotifyStartQuery {
    pub next: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/spotify/start",
    params(("next" = Option<String>, Query)),
    responses((status = 307, description = "Redirects to Spotify OAuth"))
)]
pub async fn spotify_start(
    State(state): State<AppState>,
    Query(query): Query<SpotifyStartQuery>,
) -> Result<impl IntoResponse> {
    let login = auth_service::start_spotify_login(&state, sanitize_next_path(query.next)?).await?;
    let mut response = Redirect::temporary(&login.url).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        oauth_state_cookie(&state.config, &login.state)
            .parse()
            .map_err(|err| AppError::internal(format!("invalid Set-Cookie header: {err}")))?,
    );
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/spotify/callback",
    params(("code" = Option<String>, Query), ("state" = Option<String>, Query), ("error" = Option<String>, Query)),
    responses((status = 303, description = "Sets session cookie and redirects to frontend"))
)]
pub async fn spotify_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
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
    if oauth_state_from_headers(&headers).as_deref() != Some(oauth_state.as_str()) {
        return Err(AppError::validation("invalid OAuth state cookie"));
    }
    let login = auth_service::complete_spotify_login(&state, &oauth_state, &code).await?;

    let redirect_url = callback_redirect_url(&state, login.next_path.as_deref())?;
    let mut response = Redirect::to(&redirect_url).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        login_cookie(&state.config, &login.session_token)
            .parse()
            .map_err(|err| AppError::internal(format!("invalid Set-Cookie header: {err}")))?,
    );
    response.headers_mut().append(
        SET_COOKIE,
        clear_oauth_state_cookie(&state.config)
            .parse()
            .map_err(|err| AppError::internal(format!("invalid Set-Cookie header: {err}")))?,
    );
    Ok(response)
}

fn sanitize_next_path(next: Option<String>) -> Result<Option<String>> {
    let Some(next) = next else {
        return Ok(None);
    };
    let next = next.trim();
    if next.is_empty() {
        return Ok(None);
    }
    if next.starts_with('/')
        && !next.starts_with("//")
        && !next.contains('\\')
        && !next.chars().any(char::is_control)
    {
        return Ok(Some(next.chars().take(1024).collect()));
    }
    Err(AppError::validation(
        "login next path must be a relative same-origin path",
    ))
}

fn callback_redirect_url(state: &AppState, next_path: Option<&str>) -> Result<String> {
    let Some(next_path) = next_path else {
        return Ok(state.config.client_endpoint.as_str().to_owned());
    };
    state
        .config
        .client_endpoint
        .join(next_path.trim_start_matches('/'))
        .map(|url| url.to_string())
        .map_err(|err| AppError::internal(err.to_string()))
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
    let public_sharing_enabled = public_tokens::enabled_for_user(&state.db, user.id).await?;
    Ok(Json(MeResponse {
        user: user.into(),
        settings: user_settings,
        public_sharing: crate::dto::responses::PublicSharingResponse {
            enabled: public_sharing_enabled,
            token: None,
        },
    }))
}
