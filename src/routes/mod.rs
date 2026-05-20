pub mod admin;
pub mod auth;
pub mod catalog;
pub mod health;
pub mod imports;
pub mod metrics;
pub mod public;
pub mod stats;
pub mod users;

use axum::{Router, extract::DefaultBodyLimit};

use crate::state::AppState;

pub fn api_v1(import_body_limit: usize) -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(users::router())
        .merge(admin::router())
        .merge(stats::router())
        .merge(catalog::router())
        .merge(imports::router().layer(DefaultBodyLimit::max(import_body_limit)))
        .merge(public::router())
}
