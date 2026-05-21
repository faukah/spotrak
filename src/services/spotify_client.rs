use std::{collections::HashSet, time::Instant};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::{StatusCode, header};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::{
    config::{Config, MIN_SPOTIFY_API_DELAY},
    domain::spotify::{
        SpotifyAlbum, SpotifyArtist, SpotifyProfile, SpotifyRecentlyPlayedItem,
        SpotifyRecentlyPlayedResponse, SpotifySearchTracks, SpotifyTokenResponse, SpotifyTrack,
        StoredSpotifyTokens,
    },
    error::{AppError, Result},
    state::AppState,
};

const SPOTIFY_ACCOUNTS: &str = "https://accounts.spotify.com";
const SPOTIFY_API: &str = "https://api.spotify.com";
const MAX_RETRY_AFTER_SECONDS: u64 = 60;
const MAX_SPOTIFY_RETRIES: u32 = 3;
const SPOTIFY_AUTH_SCOPES: &str = "user-read-private user-read-recently-played";
const REQUIRED_SPOTIFY_SCOPES: &[&str] = &["user-read-private", "user-read-recently-played"];

pub fn authorize_url(config: &Config, state: &str, code_challenge: &str) -> Result<Url> {
    let mut url = Url::parse(&format!("{SPOTIFY_ACCOUNTS}/authorize"))
        .map_err(|err| AppError::internal(err.to_string()))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.spotify_public)
        .append_pair("redirect_uri", &config.oauth_callback_url())
        .append_pair("scope", SPOTIFY_AUTH_SCOPES)
        .append_pair("state", state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", code_challenge);
    Ok(url)
}

pub async fn exchange_code(
    state: &AppState,
    code: &str,
    code_verifier: &str,
) -> Result<StoredSpotifyTokens> {
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
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;
    parse_token_response(response, true).await
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
    parse_token_response(response, false).await
}

pub async fn me(state: &AppState, access_token: &str) -> Result<SpotifyProfile> {
    spotify_get_json(state, &format!("{SPOTIFY_API}/v1/me"), access_token).await
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

        let page =
            match spotify_get_json::<SpotifyRecentlyPlayedResponse>(state, &page_url, access_token)
                .await
            {
                Ok(page) => page,
                Err(error) if is_spotify_status(&error, StatusCode::UNAUTHORIZED) => {
                    return Err(error);
                }
                Err(error) if !items.is_empty() => {
                    tracing::warn!(
                        ?error,
                        "returning partial Spotify recently-played page results"
                    );
                    break;
                }
                Err(error) => return Err(error),
            };
        if page.items.is_empty() {
            break;
        }
        next = page
            .next
            .as_deref()
            .map(validated_spotify_api_url)
            .transpose()?;
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

fn validated_spotify_api_url(value: &str) -> Result<String> {
    let url = Url::parse(value)
        .map_err(|err| AppError::spotify(format!("invalid Spotify pagination URL: {err}")))?;
    if url.scheme() != "https" || url.host_str() != Some("api.spotify.com") {
        return Err(AppError::spotify("invalid Spotify pagination URL origin"));
    }
    Ok(url.to_string())
}

fn append_market(state: &AppState, url: &mut Url) {
    if let Some(market) = &state.config.spotify_market {
        url.query_pairs_mut().append_pair("market", market);
    }
}

fn is_spotify_status(error: &AppError, status_code: StatusCode) -> bool {
    matches!(
        error,
        AppError::SpotifyApi { status, .. } if *status == status_code
    )
}

async fn spotify_get_json<T: DeserializeOwned>(
    state: &AppState,
    url: &str,
    access_token: &str,
) -> Result<T> {
    spotify_send(state, || state.http.get(url).bearer_auth(access_token)).await
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
    let mut attempts: u32 = 0;
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
        if status == StatusCode::TOO_MANY_REQUESTS && attempts <= MAX_SPOTIFY_RETRIES {
            let retry_after = retry_after_seconds(response.headers());
            let sleep_seconds = retry_after.unwrap_or_else(|| spotify_backoff_seconds(attempts));
            if sleep_seconds > MAX_RETRY_AFTER_SECONDS {
                state.metrics.inc_spotify_failures();
                tracing::warn!(
                    %url,
                    retry_after_seconds = retry_after.unwrap_or(0),
                    retry_after_present = retry_after.is_some(),
                    sleep_seconds,
                    max_retry_after_seconds = MAX_RETRY_AFTER_SECONDS,
                    attempt = attempts,
                    "Spotify rate limit Retry-After is too long; not sleeping the worker"
                );
                return Err(spotify_rate_limit_error(response, retry_after, sleep_seconds).await);
            }
            tracing::warn!(
                %url,
                retry_after_seconds = retry_after.unwrap_or(0),
                retry_after_present = retry_after.is_some(),
                sleep_seconds,
                attempt = attempts,
                "Spotify rate limit hit; respecting Retry-After or exponential backoff before retrying"
            );
            tokio::time::sleep(std::time::Duration::from_secs(sleep_seconds)).await;
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
    let delay = state.config.spotify_api_delay.max(MIN_SPOTIFY_API_DELAY);

    // Process-local guard first. Keep the mutex scoped only around local spacing;
    // cross-process DB coordination below uses the advisory lock.
    {
        let mut last_request_at = state.spotify_limiter.lock().await;
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
        *last_request_at = Instant::now();
    }

    // Database guard second. This makes the spacing process-wide even if two backend
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

    let mut last_request_at = state.spotify_limiter.lock().await;
    *last_request_at = Instant::now();
    Ok(())
}

async fn parse_token_response(
    response: reqwest::Response,
    validate_scopes: bool,
) -> Result<StoredSpotifyTokens> {
    if !response.status().is_success() {
        return Err(spotify_http_error(response).await);
    }
    let token = response.json::<SpotifyTokenResponse>().await?;
    if !token.token_type.eq_ignore_ascii_case("bearer") {
        return Err(AppError::spotify(
            "Spotify returned an unsupported token type",
        ));
    }
    if validate_scopes {
        validate_granted_scopes(token.scope.as_deref())?;
    }
    let expires_in = token.expires_in.max(0);
    let token_expires_at = Utc::now() + ChronoDuration::seconds(expires_in);
    Ok(StoredSpotifyTokens {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        token_expires_at,
    })
}

fn validate_granted_scopes(scope: Option<&str>) -> Result<()> {
    let granted = scope
        .unwrap_or_default()
        .split_whitespace()
        .collect::<HashSet<_>>();
    let missing = REQUIRED_SPOTIFY_SCOPES
        .iter()
        .copied()
        .filter(|required| !granted.contains(required))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(AppError::spotify(format!(
        "Spotify did not grant required scope(s): {}",
        missing.join(", ")
    )))
}

fn retry_after_seconds(headers: &header::HeaderMap) -> Option<u64> {
    let value = headers.get(header::RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }

    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map(|timestamp| {
            (timestamp - Utc::now())
                .to_std()
                .map_or(0, |duration| duration.as_secs())
        })
}

fn spotify_backoff_seconds(attempt: u32) -> u64 {
    2_u64.saturating_pow(attempt.min(5))
}

async fn spotify_rate_limit_error(
    response: reqwest::Response,
    retry_after: Option<u64>,
    sleep_seconds: u64,
) -> AppError {
    let status = response.status();
    let url = response.url().to_string();
    let body = response
        .text()
        .await
        .unwrap_or_else(|err| format!("<failed to read response body: {err}>"));
    let message = spotify_error_message(status, &body, retry_after);
    AppError::SpotifyApi {
        status,
        url,
        message,
        body: format!(
            "{body}; retry_after_seconds={}; planned_sleep_seconds={sleep_seconds}; max_retry_after_seconds={MAX_RETRY_AFTER_SECONDS}; request was not retried in-process to avoid blocking the worker",
            retry_after
                .map(|seconds| seconds.to_string())
                .unwrap_or_else(|| "<missing>".to_owned())
        ),
    }
}

async fn spotify_http_error(response: reqwest::Response) -> AppError {
    let status = response.status();
    let url = response.url().to_string();
    let retry_after = retry_after_seconds(response.headers());
    let body = response
        .text()
        .await
        .unwrap_or_else(|err| format!("<failed to read response body: {err}>"));
    let message = spotify_error_message(status, &body, retry_after);
    AppError::SpotifyApi {
        status,
        url,
        message,
        body,
    }
}

fn spotify_error_message(status: StatusCode, body: &str, retry_after: Option<u64>) -> String {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return match retry_after {
            Some(seconds) => format!("rate limited by Spotify; retry after {seconds}s"),
            None => "rate limited by Spotify; retry later".to_owned(),
        };
    }

    let parsed = serde_json::from_str::<Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/message").and_then(Value::as_str))
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|value| value.get("error_description").and_then(Value::as_str))
        })
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|value| value.get("message").and_then(Value::as_str))
        })
        .or_else(|| {
            parsed.as_ref().and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .filter(|message| !message.trim().is_empty())
            })
        })
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| status_reason(status).to_owned());

    sanitize_spotify_error_message(&message)
}

fn status_reason(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad request to Spotify",
        StatusCode::UNAUTHORIZED => "Spotify authorization failed",
        StatusCode::FORBIDDEN => "Spotify denied this operation",
        StatusCode::NOT_FOUND => "Spotify resource not found",
        StatusCode::TOO_MANY_REQUESTS => "Spotify rate limit exceeded",
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => "Spotify is temporarily unavailable",
        _ => "unexpected Spotify API response",
    }
}

fn sanitize_spotify_error_message(message: &str) -> String {
    const MAX_SPOTIFY_ERROR_MESSAGE_LEN: usize = 240;
    let mut sanitized = message
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_SPOTIFY_ERROR_MESSAGE_LEN)
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized = "unexpected Spotify API response".to_owned();
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf, time::Duration};

    use secrecy::SecretString;

    use super::*;
    use crate::config::CorsConfig;

    #[test]
    fn authorize_url_uses_least_privilege_scopes_and_pkce() {
        let config = Config {
            database_url: SecretString::from("postgresql://spotrak:spotrak@127.0.0.1/spotrak"),
            api_endpoint: Url::parse("https://api.spotrak.example").unwrap(),
            client_endpoint: Url::parse("https://spotrak.example").unwrap(),
            spotify_public: "spotify_client_id".to_owned(),
            spotify_secret: SecretString::from("spotify_client_secret"),
            spotify_token_encryption_key: SecretString::from("spotify_token_key"),
            spotify_market: None,
            port: 8080,
            timezone: chrono_tz::UTC,
            log_level: "info".to_owned(),
            cookie_validity: Duration::from_secs(60),
            spotify_api_delay: MIN_SPOTIFY_API_DELAY,
            cors: CorsConfig::Any,
            import_dir: PathBuf::from("imports"),
            max_import_cache_size: 0,
            prometheus_username: None,
            prometheus_password: None,
        };

        let url = authorize_url(&config, "raw-state", "pkce-challenge").unwrap();
        let query = url.query_pairs().into_owned().collect::<BTreeMap<_, _>>();

        assert_eq!(
            query.get("scope").map(String::as_str),
            Some(SPOTIFY_AUTH_SCOPES)
        );
        assert!(!query.get("scope").unwrap().contains("user-read-email"));
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(
            query.get("code_challenge").map(String::as_str),
            Some("pkce-challenge")
        );
        assert_eq!(query.get("state").map(String::as_str), Some("raw-state"));
    }
}
