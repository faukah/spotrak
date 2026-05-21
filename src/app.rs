use crate::{config::CorsConfig, dto::openapi::ApiDoc, error::AppError, routes, state::AppState};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Request},
    http::{HeaderName, HeaderValue, Method, StatusCode, Uri, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;

const DEFAULT_BODY_LIMIT: usize = 1024 * 1024;
const CSRF_HEADER: &str = "x-spotrak-csrf";

pub fn build(state: AppState) -> Router {
    let import_body_limit =
        usize::try_from(state.config.max_import_cache_size).unwrap_or(usize::MAX);
    let api = routes::api_v1(import_body_limit)
        .route("/openapi.json", get(openapi_json))
        .layer(middleware::from_fn(csrf_guard));

    Router::new()
        .nest("/api/v1", api)
        .merge(routes::metrics::router())
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .layer(DefaultBodyLimit::max(DEFAULT_BODY_LIMIT))
        .layer(cors_layer(&state.config.cors))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                tracing::debug_span!(
                    "request",
                    method = %request.method(),
                    uri = %redacted_request_uri(request.uri()),
                    version = ?request.version()
                )
            }),
        )
        .with_state(state)
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn not_found() -> AppError {
    AppError::NotFound
}

fn redacted_request_uri(uri: &Uri) -> String {
    let path = uri.path();
    if let Some(rest) = path.strip_prefix("/api/v1/public/") {
        return match rest.split_once('/') {
            Some((_, tail)) => format!("/api/v1/public/{{token}}/{tail}"),
            None => "/api/v1/public/{token}".to_owned(),
        };
    }
    if path.starts_with("/api/v1/auth/spotify/") {
        return path.to_owned();
    }
    uri.path_and_query()
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_owned())
}

async fn csrf_guard(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let method = request.method();
    let unsafe_method = matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if unsafe_method {
        let valid = request
            .headers()
            .get(CSRF_HEADER)
            .and_then(|value| value.to_str().ok())
            == Some("1");
        if !valid {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(request).await)
}

fn cors_layer(config: &CorsConfig) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
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
                .allow_headers([
                    header::ACCEPT,
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    HeaderName::from_static(CSRF_HEADER),
                ])
                .allow_credentials(true)
        }
    }
}
