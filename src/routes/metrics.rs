use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use secrecy::ExposeSecret;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics))
}

#[utoipa::path(
    get,
    path = "/metrics",
    responses((status = 200, description = "Prometheus metrics"), (status = 401, description = "Unauthorized"))
)]
pub async fn metrics(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"spotrak metrics\"")],
            "Authentication required".to_owned(),
        )
            .into_response();
    }

    let version = env!("CARGO_PKG_VERSION");
    let snapshot = state.metrics.snapshot();
    let body = format!(
        "# HELP spotrak_build_info Build metadata for the Spotrak backend\n\
         # TYPE spotrak_build_info gauge\n\
         spotrak_build_info{{version=\"{version}\"}} 1\n\
         # HELP spotrak_spotify_requests_total Spotify API requests made by the backend\n\
         # TYPE spotrak_spotify_requests_total counter\n\
         spotrak_spotify_requests_total {}\n\
         # HELP spotrak_spotify_failures_total Failed Spotify API responses\n\
         # TYPE spotrak_spotify_failures_total counter\n\
         spotrak_spotify_failures_total {}\n\
         # HELP spotrak_import_jobs_processed_total Import jobs picked up by the worker\n\
         # TYPE spotrak_import_jobs_processed_total counter\n\
         spotrak_import_jobs_processed_total {}\n",
        snapshot.spotify_requests_total,
        snapshot.spotify_failures_total,
        snapshot.import_jobs_processed_total,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
        .into_response()
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let (Some(expected_username), Some(expected_password)) = (
        state.config.prometheus_username.as_deref(),
        state.config.prometheus_password.as_ref(),
    ) else {
        return false;
    };

    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(credentials) = String::from_utf8(decoded) else {
        return false;
    };
    let Some((username, password)) = credentials.split_once(':') else {
        return false;
    };

    constant_time_eq(username.as_bytes(), expected_username.as_bytes())
        & constant_time_eq(
            password.as_bytes(),
            expected_password.expose_secret().as_bytes(),
        )
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}
