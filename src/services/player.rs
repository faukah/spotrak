use std::sync::Arc;

use chrono::{Duration, Utc};
use reqwest::StatusCode;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    domain::{
        player::{
            CurrentlyPlayingResponse, CurrentlyPlayingTrack, CurrentlyPlayingUnavailableReason,
        },
        spotify::{SpotifyCurrentlyPlayingResponse, SpotifyTrack},
    },
    error::{AppError, Result},
    repositories::{response_cache, users},
    services::{poller, spotify_client},
    state::AppState,
};

const CURRENTLY_PLAYING_CACHE_KEY: &str = "v1";
const CURRENTLY_PLAYING_TTL: Duration = Duration::seconds(12);
const CURRENTLY_PLAYING_FAILURE_TTL: Duration = Duration::seconds(20);

pub async fn currently_playing_for_user(
    state: &AppState,
    user_id: Uuid,
) -> Result<CurrentlyPlayingResponse> {
    if let Some(cached) = cached_currently_playing(state, user_id).await? {
        return Ok(cached);
    }

    let lock = currently_playing_lock(state, user_id).await;
    let _guard = lock.lock().await;

    if let Some(cached) = cached_currently_playing(state, user_id).await? {
        return Ok(cached);
    }

    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let mut access_token = poller::valid_access_token(state, &user).await?;
    let live = match spotify_client::currently_playing(state, &access_token).await {
        Ok(live) => live,
        Err(error) if poller::is_spotify_authorization_error(&error) => {
            access_token = poller::force_refresh_access_token(state, user_id).await?;
            spotify_client::currently_playing(state, &access_token).await?
        }
        Err(error) if is_reconnect_required(&error) => {
            let response = unavailable(CurrentlyPlayingUnavailableReason::ReconnectRequired);
            cache_currently_playing(state, user_id, &response, CURRENTLY_PLAYING_FAILURE_TTL)
                .await?;
            return Ok(response);
        }
        Err(error) if is_spotify_rate_limited(&error) => {
            let response = unavailable(CurrentlyPlayingUnavailableReason::SpotifyUnavailable);
            cache_currently_playing(state, user_id, &response, CURRENTLY_PLAYING_FAILURE_TTL)
                .await?;
            return Ok(response);
        }
        Err(error) => return Err(error),
    };

    let response = live
        .map(currently_playing_response)
        .transpose()?
        .unwrap_or_else(|| unavailable(CurrentlyPlayingUnavailableReason::NotPlaying));

    cache_currently_playing(state, user_id, &response, CURRENTLY_PLAYING_TTL).await?;
    Ok(response)
}

async fn cached_currently_playing(
    state: &AppState,
    user_id: Uuid,
) -> Result<Option<CurrentlyPlayingResponse>> {
    response_cache::get(
        &state.db,
        response_cache::CURRENTLY_PLAYING_NAMESPACE,
        user_id,
        CURRENTLY_PLAYING_CACHE_KEY,
    )
    .await
}

async fn currently_playing_lock(state: &AppState, user_id: Uuid) -> Arc<Mutex<()>> {
    let mut locks = state.currently_playing_locks.lock().await;
    locks
        .entry(user_id)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn currently_playing_response(
    live: SpotifyCurrentlyPlayingResponse,
) -> Result<CurrentlyPlayingResponse> {
    let Some(item) = live.item else {
        return Ok(unavailable(CurrentlyPlayingUnavailableReason::NotPlaying));
    };

    if !matches!(live.currently_playing_type.as_deref(), Some("track") | None) {
        return Ok(unavailable(
            CurrentlyPlayingUnavailableReason::UnsupportedItem,
        ));
    }

    let track = serde_json::from_value::<SpotifyTrack>(item)
        .map_err(|err| AppError::spotify(format!("invalid Spotify playback track: {err}")))?;

    Ok(CurrentlyPlayingResponse {
        fetched_at: Utc::now(),
        is_playing: live.is_playing,
        progress_ms: live.progress_ms,
        track: Some(currently_playing_track(track)?),
        unavailable_reason: None,
    })
}

fn currently_playing_track(track: SpotifyTrack) -> Result<CurrentlyPlayingTrack> {
    let id = track
        .id
        .ok_or_else(|| AppError::spotify("Spotify playback track did not include an id"))?;
    Ok(CurrentlyPlayingTrack {
        id,
        name: track.name,
        album_id: track.album.id,
        album_name: track.album.name,
        artist_name: artist_name(&track.artists),
        image_url: track.album.images.first().map(|image| image.url.clone()),
        duration_ms: track.duration_ms,
    })
}

fn artist_name(artists: &[crate::domain::spotify::SpotifySimpleArtist]) -> Option<String> {
    let names = artists
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if names.is_empty() {
        None
    } else {
        Some(names.join(", "))
    }
}

fn unavailable(reason: CurrentlyPlayingUnavailableReason) -> CurrentlyPlayingResponse {
    CurrentlyPlayingResponse {
        fetched_at: Utc::now(),
        is_playing: false,
        progress_ms: None,
        track: None,
        unavailable_reason: Some(reason),
    }
}

async fn cache_currently_playing(
    state: &AppState,
    user_id: Uuid,
    response: &CurrentlyPlayingResponse,
    ttl: Duration,
) -> Result<()> {
    response_cache::set(
        &state.db,
        response_cache::CURRENTLY_PLAYING_NAMESPACE,
        user_id,
        CURRENTLY_PLAYING_CACHE_KEY,
        response,
        Some(ttl),
    )
    .await
}

fn is_reconnect_required(error: &AppError) -> bool {
    matches!(
        error,
        AppError::SpotifyApi { status, .. } if *status == StatusCode::FORBIDDEN
    )
}

fn is_spotify_rate_limited(error: &AppError) -> bool {
    matches!(
        error,
        AppError::SpotifyApi { status, .. } if *status == StatusCode::TOO_MANY_REQUESTS
    )
}
