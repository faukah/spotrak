use crate::{config::CorsConfig, dto::openapi::ApiDoc, error::AppError, routes, state::AppState};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::get,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

pub fn build(state: AppState) -> Router {
    let api = routes::api_v1().route("/openapi.json", get(openapi_json));
    let body_limit = usize::try_from(state.config.max_import_cache_size).unwrap_or(usize::MAX);

    Router::new()
        .nest("/api/v1", api)
        .merge(routes::metrics::router())
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(cors_layer(&state.config.cors))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn not_found() -> AppError {
    AppError::NotFound
}

fn cors_layer(config: &CorsConfig) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];
    match config {
        CorsConfig::Any => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(methods)
            .allow_headers(Any),
        CorsConfig::Origins(origins) => {
            let values = origins
                .iter()
                .filter_map(|origin| {
                    origin
                        .origin()
                        .ascii_serialization()
                        .parse::<HeaderValue>()
                        .ok()
                })
                .collect::<Vec<_>>();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(values))
                .allow_methods(methods)
                .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE])
                .allow_credentials(true)
        }
    }
}
