pub mod admin;
pub mod auth;
pub mod catalog;
pub mod health;
pub mod imports;
pub mod metrics;
pub mod public;
pub mod settings;
pub mod spotify;
pub mod stats;
pub mod users;

use axum::Router;

use crate::state::AppState;

pub fn api_v1() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(users::router())
        .merge(settings::router())
        .merge(admin::router())
        .merge(stats::router())
        .merge(catalog::router())
        .merge(imports::router())
        .merge(spotify::router())
        .merge(public::router())
}
