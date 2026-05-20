use std::time::Instant;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::{StatusCode, header};
use secrecy::ExposeSecret;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use url::Url;

use crate::{
    config::Config,
    domain::spotify::{
        SpotifyAlbum, SpotifyArtist, SpotifyCurrentlyPlaying, SpotifyPlaylistSummary,
        SpotifyPlaylistsResponse, SpotifyProfile, SpotifyRecentlyPlayedItem,
        SpotifyRecentlyPlayedResponse, SpotifySearchTracks, SpotifyTokenResponse, SpotifyTrack,
        StoredSpotifyTokens,
    },
    error::{AppError, Result},
    state::AppState,
};

const SPOTIFY_ACCOUNTS: &str = "https://accounts.spotify.com";
const SPOTIFY_API: &str = "https://api.spotify.com";
const MAX_RETRY_AFTER_SECONDS: u64 = 60;
const MIN_SPOTIFY_REQUEST_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

pub fn authorize_url(config: &Config, state: &str) -> Result<Url> {
    let mut url = Url::parse(&format!("{SPOTIFY_ACCOUNTS}/authorize"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.spotify_public)
        .append_pair("redirect_uri", &config.oauth_callback_url())
        .append_pair("scope", "user-read-email user-read-private user-read-recently-played user-read-currently-playing user-read-playback-state playlist-read-private playlist-modify-public playlist-modify-private user-modify-playback-state")
        .append_pair("state", state);
    Ok(url)
}

pub async fn exchange_code(state: &AppState, code: &str) -> Result<StoredSpotifyTokens> {
    wait_for_spotify_slot(state).await?;
    state.metrics.inc_spotify_requests();
    let response = state
        .http
        .post(format!("{SPOTIFY_ACCOUNTS}/api/token"))
        .basic_auth(
            &state.config.spotify_public,
            Some(state.config.spotify_secret.expose_secret()),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &state.config.oauth_callback_url()),
        ])
        .send()
        .await?;
    parse_token_response(response).await
}

pub async fn refresh_token(state: &AppState, refresh_token: &str) -> Result<StoredSpotifyTokens> {
    wait_for_spotify_slot(state).await?;
    state.metrics.inc_spotify_requests();
    let response = state
        .http
        .post(format!("{SPOTIFY_ACCOUNTS}/api/token"))
        .basic_auth(
            &state.config.spotify_public,
            Some(state.config.spotify_secret.expose_secret()),
        )
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;
    parse_token_response(response).await
}

pub async fn me(state: &AppState, access_token: &str) -> Result<SpotifyProfile> {
    spotify_get_json(state, &format!("{SPOTIFY_API}/v1/me"), access_token).await
}

pub async fn currently_playing(
    state: &AppState,
    access_token: &str,
) -> Result<Option<SpotifyCurrentlyPlaying>> {
    spotify_get_json(
        state,
        &format!("{SPOTIFY_API}/v1/me/player/currently-playing"),
        access_token,
    )
    .await
}

pub async fn recently_played_after(
    state: &AppState,
    access_token: &str,
    after: Option<DateTime<Utc>>,
) -> Result<Vec<SpotifyRecentlyPlayedItem>> {
    let mut url = Url::parse(&format!("{SPOTIFY_API}/v1/me/player/recently-played"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    url.query_pairs_mut().append_pair("limit", "50");
    if let Some(after) = after {
        url.query_pairs_mut()
            .append_pair("after", &after.timestamp_millis().to_string());
    }

    let mut items = Vec::new();
    let mut next = Some(url.to_string());
    let mut pages = 0;

    while let Some(page_url) = next {
        pages += 1;
        if pages > 10 {
            break;
        }

        let page: SpotifyRecentlyPlayedResponse =
            spotify_get_json(state, &page_url, access_token).await?;
        if page.items.is_empty() {
            break;
        }
        next = page.next.clone();
        items.extend(page.items);
    }

    Ok(items)
}

pub async fn get_track(state: &AppState, access_token: &str, id: &str) -> Result<SpotifyTrack> {
    let mut url = Url::parse(&format!("{SPOTIFY_API}/v1/tracks/{id}"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    append_market(state, &mut url);
    spotify_get_json(state, url.as_str(), access_token).await
}

#[allow(dead_code)]
pub async fn get_album(state: &AppState, access_token: &str, id: &str) -> Result<SpotifyAlbum> {
    let mut url = Url::parse(&format!("{SPOTIFY_API}/v1/albums/{id}"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    append_market(state, &mut url);
    spotify_get_json(state, url.as_str(), access_token).await
}

#[allow(dead_code)]
pub async fn get_artist(state: &AppState, access_token: &str, id: &str) -> Result<SpotifyArtist> {
    spotify_get_json(
        state,
        &format!("{SPOTIFY_API}/v1/artists/{id}"),
        access_token,
    )
    .await
}

pub async fn search_track(
    state: &AppState,
    access_token: &str,
    query: &str,
) -> Result<Option<SpotifyTrack>> {
    let mut url = Url::parse(&format!("{SPOTIFY_API}/v1/search"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    url.query_pairs_mut()
        .append_pair("type", "track")
        .append_pair("limit", "1")
        .append_pair("q", query);
    append_market(state, &mut url);
    let response: SpotifySearchTracks = spotify_get_json(state, url.as_str(), access_token).await?;
    Ok(response.tracks.items.into_iter().next())
}

pub async fn play_track(state: &AppState, access_token: &str, track_uri: &str) -> Result<()> {
    spotify_put_json_no_content(
        state,
        &format!("{SPOTIFY_API}/v1/me/player/play"),
        access_token,
        &json!({ "uris": [track_uri] }),
    )
    .await
}

pub async fn get_playlists(
    state: &AppState,
    access_token: &str,
) -> Result<Vec<SpotifyPlaylistSummary>> {
    let mut url = Url::parse(&format!("{SPOTIFY_API}/v1/me/playlists"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    url.query_pairs_mut().append_pair("limit", "50");
    let mut playlists = Vec::new();
    let mut next = Some(url.to_string());
    let mut pages = 0;
    while let Some(page_url) = next {
        pages += 1;
        if pages > 10 {
            break;
        }
        let page: SpotifyPlaylistsResponse =
            spotify_get_json(state, &page_url, access_token).await?;
        next = page.next.clone();
        playlists.extend(page.items);
    }
    Ok(playlists)
}

pub async fn create_playlist(
    state: &AppState,
    access_token: &str,
    user_id: &str,
    name: &str,
    public: bool,
) -> Result<SpotifyPlaylistSummary> {
    spotify_post_json(
        state,
        &format!("{SPOTIFY_API}/v1/users/{user_id}/playlists"),
        access_token,
        &json!({ "name": name, "public": public }),
    )
    .await
}

pub async fn add_tracks_to_playlist(
    state: &AppState,
    access_token: &str,
    playlist_id: &str,
    uris: &[String],
) -> Result<()> {
    spotify_post_json::<Value, _>(
        state,
        &format!("{SPOTIFY_API}/v1/playlists/{playlist_id}/tracks"),
        access_token,
        &json!({ "uris": uris }),
    )
    .await
    .map(|_| ())
}

fn append_market(state: &AppState, url: &mut Url) {
    if let Some(market) = &state.config.spotify_market {
        url.query_pairs_mut().append_pair("market", market);
    }
}

async fn spotify_get_json<T: DeserializeOwned>(
    state: &AppState,
    url: &str,
    access_token: &str,
) -> Result<T> {
    spotify_send(state, || state.http.get(url).bearer_auth(access_token)).await
}

async fn spotify_post_json<T: DeserializeOwned, B: Serialize + ?Sized>(
    state: &AppState,
    url: &str,
    access_token: &str,
    body: &B,
) -> Result<T> {
    spotify_send(state, || {
        state.http.post(url).bearer_auth(access_token).json(body)
    })
    .await
}

async fn spotify_put_json_no_content<B: Serialize + ?Sized>(
    state: &AppState,
    url: &str,
    access_token: &str,
    body: &B,
) -> Result<()> {
    let _: Value = spotify_send_allow_empty(state, || {
        state.http.put(url).bearer_auth(access_token).json(body)
    })
    .await?;
    Ok(())
}

async fn spotify_send<T: DeserializeOwned>(
    state: &AppState,
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<T> {
    spotify_send_allow_empty(state, build).await
}

async fn spotify_send_allow_empty<T: DeserializeOwned>(
    state: &AppState,
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<T> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        wait_for_spotify_slot(state).await?;
        state.metrics.inc_spotify_requests();
        let started_at = Instant::now();
        let response = build().send().await?;
        let elapsed_ms = started_at.elapsed().as_millis();
        let status = response.status();
        let url = response.url().to_string();
        tracing::debug!(%status, %url, elapsed_ms, attempt = attempts, "Spotify request completed");
        if status == StatusCode::TOO_MANY_REQUESTS && attempts <= 3 {
            let retry_after = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1);
            if retry_after > MAX_RETRY_AFTER_SECONDS {
                state.metrics.inc_spotify_failures();
                tracing::warn!(
                    %url,
                    retry_after_seconds = retry_after,
                    max_retry_after_seconds = MAX_RETRY_AFTER_SECONDS,
                    attempt = attempts,
                    "Spotify rate limit Retry-After is too long; not sleeping the worker"
                );
                return Err(spotify_rate_limit_error(response, retry_after).await);
            }
            tracing::warn!(
                %url,
                retry_after_seconds = retry_after,
                attempt = attempts,
                "Spotify rate limit hit; respecting Retry-After before retrying"
            );
            tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
            continue;
        }

        if !status.is_success() {
            state.metrics.inc_spotify_failures();
            return Err(spotify_http_error(response).await);
        }

        if status == StatusCode::NO_CONTENT {
            return serde_json::from_value(Value::Null)
                .map_err(|err| AppError::internal(err.to_string()));
        }

        return Ok(response.json::<T>().await?);
    }
}

async fn wait_for_spotify_slot(state: &AppState) -> Result<()> {
    let mut last_request_at = state.spotify_limiter.lock().await;
    let delay = state
        .config
        .spotify_api_delay
        .max(MIN_SPOTIFY_REQUEST_DELAY);

    // Process-local guard first. This protects all tasks within this backend process.
    let now = Instant::now();
    let next_allowed_at = *last_request_at + delay;
    if next_allowed_at > now {
        let wait = next_allowed_at - now;
        tracing::debug!(
            wait_ms = wait.as_millis(),
            "waiting for local Spotify rate limiter slot"
        );
        tokio::time::sleep(wait).await;
    }

    // Database guard second. This makes the 2s spacing process-wide even if two backend
    // instances or a restarted worker overlap. The transaction intentionally holds the
    // advisory lock while sleeping so no other process can reserve the same slot.
    let mut tx = state.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(7_764_786_970_019_i64)
        .execute(&mut *tx)
        .await?;

    let last_global = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT value_ts FROM app_runtime_state WHERE key = 'spotify_last_request_at' FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if let Some(last_global) = last_global {
        let chrono_delay =
            ChronoDuration::from_std(delay).map_err(|err| AppError::internal(err.to_string()))?;
        let next_global = last_global + chrono_delay;
        let now_global = Utc::now();
        if next_global > now_global {
            let wait = (next_global - now_global)
                .to_std()
                .map_err(|err| AppError::internal(err.to_string()))?;
            tracing::debug!(
                wait_ms = wait.as_millis(),
                "waiting for global Spotify rate limiter slot"
            );
            tokio::time::sleep(wait).await;
        }
    }

    let reserved_at = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO app_runtime_state (key, value_ts, updated_at)
        VALUES ('spotify_last_request_at', $1, now())
        ON CONFLICT (key) DO UPDATE SET value_ts = EXCLUDED.value_ts, updated_at = now()
        "#,
    )
    .bind(reserved_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    *last_request_at = Instant::now();
    Ok(())
}

async fn parse_token_response(response: reqwest::Response) -> Result<StoredSpotifyTokens> {
    if !response.status().is_success() {
        return Err(spotify_http_error(response).await);
    }
    let token = response.json::<SpotifyTokenResponse>().await?;
    let expires_in = token.expires_in.max(0);
    let token_expires_at = Utc::now() + ChronoDuration::seconds(expires_in);
    Ok(StoredSpotifyTokens {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        token_expires_at,
    })
}

async fn spotify_rate_limit_error(response: reqwest::Response, retry_after: u64) -> AppError {
    let status = response.status();
    let url = response.url().to_string();
    let body = response
        .text()
        .await
        .unwrap_or_else(|err| format!("<failed to read response body: {err}>"));
    AppError::SpotifyApi {
        status,
        url,
        body: format!(
            "{body}; retry_after_seconds={retry_after}; max_retry_after_seconds={MAX_RETRY_AFTER_SECONDS}; request was not retried in-process to avoid blocking the worker"
        ),
    }
}

async fn spotify_http_error(response: reqwest::Response) -> AppError {
    let status = response.status();
    let url = response.url().to_string();
    let body = response
        .text()
        .await
        .unwrap_or_else(|err| format!("<failed to read response body: {err}>"));
    AppError::SpotifyApi { status, url, body }
}
