use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};

use crate::{
    auth::extractors::current_user, domain::player::CurrentlyPlayingResponse, error::Result,
    services::player as player_service, state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/player/currently-playing", get(currently_playing))
}

#[utoipa::path(
    get,
    path = "/api/v1/player/currently-playing",
    responses((status = 200, description = "Currently playing Spotify track", body = CurrentlyPlayingResponse))
)]
pub async fn currently_playing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CurrentlyPlayingResponse>> {
    let user = current_user(&headers, &state).await?;
    Ok(Json(
        player_service::currently_playing_for_user(&state, user.id).await?,
    ))
}
