use axum::{Json, Router, routing::get};

use crate::dto::responses::{HealthResponse, VersionResponse};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses((status = 200, description = "Backend is healthy", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/api/v1/version",
    responses((status = 200, description = "Backend version", body = VersionResponse))
)]
pub async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
    })
}
