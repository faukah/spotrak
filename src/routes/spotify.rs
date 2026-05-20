use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};

use crate::{
    auth::extractors::current_user,
    domain::spotify::{SpotifyCurrentlyPlaying, SpotifyPlaylistSummary},
    dto::requests::{SpotifyAddTracksRequest, SpotifyCreatePlaylistRequest, SpotifyPlayRequest},
    error::{AppError, Result},
    services::{poller, spotify_client},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/spotify/currently-playing", get(currently_playing))
        .route("/spotify/play", post(play))
        .route("/spotify/playlists", get(playlists).post(create_playlist))
        .route(
            "/spotify/playlists/{id}/tracks",
            post(add_tracks_to_playlist),
        )
}

#[utoipa::path(
    get,
    path = "/api/v1/spotify/currently-playing",
    responses((status = 200, description = "Currently playing track", body = Option<SpotifyCurrentlyPlaying>))
)]
pub async fn currently_playing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Option<SpotifyCurrentlyPlaying>>> {
    let user = current_user(&headers, &state).await?;
    let access_token = poller::valid_access_token(&state, &user).await?;
    Ok(Json(
        spotify_client::currently_playing(&state, &access_token).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/spotify/play",
    request_body = SpotifyPlayRequest,
    responses((status = 204, description = "Playback started"))
)]
pub async fn play(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SpotifyPlayRequest>,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    let access_token = poller::valid_access_token(&state, &user).await?;
    let uri = match (request.uri, request.track_id) {
        (Some(uri), _) => uri,
        (None, Some(id)) => format!("spotify:track:{id}"),
        (None, None) => return Err(AppError::validation("uri or track_id is required")),
    };
    spotify_client::play_track(&state, &access_token, &uri).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/spotify/playlists",
    responses((status = 200, description = "Spotify playlists", body = Vec<SpotifyPlaylistSummary>))
)]
pub async fn playlists(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SpotifyPlaylistSummary>>> {
    let user = current_user(&headers, &state).await?;
    let access_token = poller::valid_access_token(&state, &user).await?;
    Ok(Json(
        spotify_client::get_playlists(&state, &access_token).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/spotify/playlists",
    request_body = SpotifyCreatePlaylistRequest,
    responses((status = 200, description = "Created Spotify playlist", body = SpotifyPlaylistSummary))
)]
pub async fn create_playlist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SpotifyCreatePlaylistRequest>,
) -> Result<Json<SpotifyPlaylistSummary>> {
    if request.name.trim().is_empty() {
        return Err(AppError::validation("playlist name must not be empty"));
    }
    let user = current_user(&headers, &state).await?;
    let access_token = poller::valid_access_token(&state, &user).await?;
    let playlist = spotify_client::create_playlist(
        &state,
        &access_token,
        &user.spotify_id,
        request.name.trim(),
        request.public,
    )
    .await?;
    Ok(Json(playlist))
}

#[utoipa::path(
    post,
    path = "/api/v1/spotify/playlists/{id}/tracks",
    request_body = SpotifyAddTracksRequest,
    responses((status = 204, description = "Tracks added to Spotify playlist"))
)]
pub async fn add_tracks_to_playlist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<SpotifyAddTracksRequest>,
) -> Result<axum::http::StatusCode> {
    if request.uris.is_empty() {
        return Err(AppError::validation("uris must not be empty"));
    }
    let user = current_user(&headers, &state).await?;
    let access_token = poller::valid_access_token(&state, &user).await?;
    spotify_client::add_tracks_to_playlist(&state, &access_token, &id, &request.uris).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
