use axum::{Router, extract::State, response::IntoResponse, routing::get};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics))
}

#[utoipa::path(
    get,
    path = "/metrics",
    responses((status = 200, description = "Prometheus metrics"))
)]
pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
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
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}
