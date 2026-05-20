use std::collections::HashMap;

use chrono_tz::Tz;

use crate::{
    domain::{
        import::ImportJob,
        spotify::{SpotifyRecentlyPlayedItem, SpotifyTrack},
    },
    error::{AppError, Result},
    repositories::{catalog, imports, response_cache, settings, spotify_queue, users},
    services::{ingestion, poller, spotify_client},
    state::AppState,
};
use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
struct ImportCandidate {
    played_at: chrono::DateTime<Utc>,
    track_id: Option<String>,
    query: Option<String>,
    track_name: Option<String>,
    artist_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrivacyEntry {
    #[serde(rename = "endTime")]
    end_time: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "msPlayed")]
    ms_played: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FullPrivacyEntry {
    ts: Option<chrono::DateTime<Utc>>,
    ms_played: Option<i32>,
    master_metadata_track_name: Option<String>,
    master_metadata_album_artist_name: Option<String>,
    spotify_track_uri: Option<String>,
}

pub async fn process_queued_once(state: &AppState) -> Result<usize> {
    let Some(job) = imports::claim_next(&state.db).await? else {
        tracing::debug!("import worker found no queued or stale import jobs");
        return Ok(0);
    };

    tracing::info!(
        job_id = %job.id,
        user_id = %job.user_id,
        import_type = %job.import_type,
        "claimed import job"
    );

    if let Err(error) = process_job(state, job.clone()).await {
        tracing::warn!(?error, job_id = %job.id, "import job failed");
        imports::mark_failure(&state.db, job.id, &error.to_string()).await?;
    }
    Ok(1)
}

pub async fn process_job(state: &AppState, job: ImportJob) -> Result<()> {
    imports::mark_progress(&state.db, job.id, 0, 0).await?;
    tracing::info!(job_id = %job.id, "starting import job");

    let files = imports::files(&state.db, job.id).await?;
    if files.is_empty() {
        return Err(AppError::validation("import job has no files"));
    }

    let user_settings = settings::get(&state.db, job.user_id).await?;
    let import_timezone = user_settings
        .timezone
        .as_deref()
        .unwrap_or_else(|| state.config.timezone.name())
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))?;

    let mut candidates = Vec::new();
    for file in files {
        let bytes = tokio::fs::read(&file.path).await.map_err(|err| {
            AppError::internal(format!("failed to read {}: {err}", file.original_name))
        })?;
        let parsed = match job.import_type.as_str() {
            "privacy" => parse_privacy_file(&bytes, import_timezone)?,
            "full-privacy" => parse_full_privacy_file(&bytes)?,
            other => {
                return Err(AppError::validation(format!(
                    "unsupported import type {other}"
                )));
            }
        };
        candidates.extend(parsed);
    }

    let total = candidates.len() as i32;
    imports::mark_progress(&state.db, job.id, total, 0).await?;
    tracing::info!(job_id = %job.id, total, "parsed import candidates");

    let user = users::find_by_id(&state.db, job.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let (access_token, token_source) = (
        poller::valid_access_token(state, &user).await?,
        "valid_user_token",
    );
    tracing::info!(
        job_id = %job.id,
        user_id = %user.id,
        token_source,
        spotify_market = ?state.config.spotify_market,
        "import metadata lookups will use Spotify user access token and single-item Spotify endpoints"
    );

    let mut cache = HashMap::<String, Option<SpotifyTrack>>::new();
    let source = match job.import_type.as_str() {
        "privacy" => "privacy-import",
        _ => "full-privacy-import",
    };
    let mut pending = Vec::<SpotifyRecentlyPlayedItem>::new();
    let mut current = 0_i32;
    for chunk in candidates.chunks(250) {
        if imports::get_any(&state.db, job.id).await?.status == "cancelled" {
            return Ok(());
        }

        for candidate in chunk {
            if current % 25 == 0 && imports::get_any(&state.db, job.id).await?.status == "cancelled"
            {
                return Ok(());
            }

            if let Some(track_id) = candidate.track_id.as_deref() {
                if catalog::has_user_track_play_near(
                    &state.db,
                    job.user_id,
                    track_id,
                    candidate.played_at,
                )
                .await?
                {
                    current += 1;
                    record_progress(state, job.id, total, current, pending.len(), cache.len())
                        .await?;
                    continue;
                }
            }

            if current == 0 || current % 100 == 0 {
                tracing::debug!(
                    job_id = %job.id,
                    next = current + 1,
                    total,
                    cache_size = cache.len(),
                    "resolving import metadata"
                );
            }

            if let Some(track) =
                resolve_track(state, &access_token, token_source, candidate, &mut cache).await?
            {
                pending.push(SpotifyRecentlyPlayedItem {
                    track,
                    played_at: candidate.played_at,
                });
            }

            current += 1;
            if pending.len() >= 250 {
                insert_items(state, job.user_id, job.id, source, &mut pending).await?;
            }
            record_progress(state, job.id, total, current, pending.len(), cache.len()).await?;
        }
    }
    insert_items(state, job.user_id, job.id, source, &mut pending).await?;
    imports::mark_success(&state.db, job.id, total, current).await?;

    Ok(())
}

async fn record_progress(
    state: &AppState,
    job_id: uuid::Uuid,
    total: i32,
    current: i32,
    pending: usize,
    cache_size: usize,
) -> Result<()> {
    if current <= 10 || current % 10 == 0 || current == total {
        imports::mark_progress(&state.db, job_id, total, current).await?;
        tracing::debug!(
            job_id = %job_id,
            current,
            total,
            pending,
            cache_size,
            "import progress"
        );
    }
    Ok(())
}

async fn insert_items(
    state: &AppState,
    user_id: uuid::Uuid,
    import_job_id: uuid::Uuid,
    source: &str,
    pending: &mut Vec<SpotifyRecentlyPlayedItem>,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let artist_ids = ingestion::artist_ids_from_recently_played(pending);
    let mut tx = state.db.begin().await?;
    let mut inserted = 0;
    for item in pending.drain(..) {
        if catalog::upsert_recently_played_event(
            &mut tx,
            user_id,
            &item,
            source,
            Some(import_job_id),
        )
        .await?
        {
            inserted += 1;
        }
    }
    tx.commit().await?;

    if inserted > 0 {
        response_cache::invalidate_namespace(
            &state.db,
            response_cache::STATS_OVERVIEW_NAMESPACE,
            user_id,
        )
        .await?;
        let missing = catalog::artists_missing_images(&state.db, &artist_ids).await?;
        spotify_queue::enqueue_artist_hydration(&state.db, user_id, &missing).await?;
    }

    Ok(())
}

fn parse_privacy_file(bytes: &[u8], timezone: Tz) -> Result<Vec<ImportCandidate>> {
    let entries = serde_json::from_slice::<Vec<PrivacyEntry>>(bytes)
        .map_err(|err| AppError::validation(format!("invalid privacy JSON: {err}")))?;
    let mut candidates = Vec::new();
    for entry in entries {
        if entry.ms_played.unwrap_or_default() <= 0 {
            continue;
        }
        let Some(end_time) = entry.end_time else {
            continue;
        };
        let Some(track_name) = entry.track_name.filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let Some(artist_name) = entry.artist_name.filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let played_at = parse_privacy_time(&end_time, timezone)?;
        candidates.push(ImportCandidate {
            played_at,
            track_id: None,
            query: Some(format!("track:{track_name} artist:{artist_name}")),
            track_name: Some(track_name),
            artist_name: Some(artist_name),
        });
    }
    Ok(candidates)
}

fn parse_full_privacy_file(bytes: &[u8]) -> Result<Vec<ImportCandidate>> {
    let entries = serde_json::from_slice::<Vec<FullPrivacyEntry>>(bytes)
        .map_err(|err| AppError::validation(format!("invalid full privacy JSON: {err}")))?;
    let mut candidates = Vec::new();
    for entry in entries {
        if entry.ms_played.unwrap_or_default() <= 0 {
            continue;
        }
        let Some(played_at) = entry.ts else {
            continue;
        };
        let track_id = entry
            .spotify_track_uri
            .as_deref()
            .and_then(track_id_from_uri);
        let (track_name, artist_name, query) = match (
            entry.master_metadata_track_name.as_ref(),
            entry.master_metadata_album_artist_name.as_ref(),
        ) {
            (Some(track), Some(artist))
                if !track.trim().is_empty() && !artist.trim().is_empty() =>
            {
                (
                    Some(track.clone()),
                    Some(artist.clone()),
                    Some(format!("track:{track} artist:{artist}")),
                )
            }
            _ => (None, None, None),
        };
        if track_id.is_none() && query.is_none() {
            continue;
        }
        candidates.push(ImportCandidate {
            played_at,
            track_id,
            query,
            track_name,
            artist_name,
        });
    }
    Ok(candidates)
}

async fn resolve_track(
    state: &AppState,
    access_token: &str,
    token_source: &'static str,
    candidate: &ImportCandidate,
    cache: &mut HashMap<String, Option<SpotifyTrack>>,
) -> Result<Option<SpotifyTrack>> {
    if let Some(track_id) = &candidate.track_id {
        let key = format!("id:{track_id}");
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
        if let Some(track) = catalog::spotify_track_from_cache(&state.db, track_id).await? {
            tracing::debug!(
                track_id,
                "using cached Spotify track metadata from database"
            );
            cache.insert(key, Some(track.clone()));
            return Ok(Some(track));
        }
        let track = match spotify_client::get_track(state, access_token, track_id).await {
            Ok(track) => Some(track),
            Err(error) => {
                if is_spotify_rate_limited(&error) {
                    tracing::warn!(
                        ?error,
                        track_id,
                        spotify_market = ?state.config.spotify_market,
                        token_source,
                        "Spotify track metadata lookup was rate-limited; stopping this import so it can be retried later"
                    );
                    return Err(error);
                }
                tracing::warn!(
                    ?error,
                    track_id,
                    spotify_market = ?state.config.spotify_market,
                    token_source,
                    "single Spotify /v1/tracks/{{id}} lookup failed during import; skipping this row. Check status/url/body in the error."
                );
                None
            }
        };
        cache.insert(key, track.clone());
        return Ok(track);
    }

    if let Some(query) = &candidate.query {
        let key = format!("q:{query}");
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
        if let (Some(track_name), Some(artist_name)) =
            (&candidate.track_name, &candidate.artist_name)
        {
            if let Some(track) =
                catalog::spotify_track_from_name_artist_cache(&state.db, track_name, artist_name)
                    .await?
            {
                tracing::debug!(
                    track_name,
                    artist_name,
                    "using cached Spotify search result from database"
                );
                cache.insert(key, Some(track.clone()));
                return Ok(Some(track));
            }
        }
        let query_key = normalize_query_key(query);
        if let Some(hit) = catalog::spotify_search_cache(&state.db, &query_key).await? {
            match hit {
                catalog::SpotifySearchCacheHit::Found(track) => {
                    tracing::debug!(query, "using cached Spotify search result");
                    cache.insert(key, Some(track.clone()));
                    return Ok(Some(track));
                }
                catalog::SpotifySearchCacheHit::NotFound => {
                    tracing::debug!(query, "using cached negative Spotify search result");
                    cache.insert(key, None);
                    return Ok(None);
                }
            }
        }
        let track = match spotify_client::search_track(state, access_token, query).await {
            Ok(track) => track,
            Err(error) => {
                if is_spotify_rate_limited(&error) {
                    tracing::warn!(
                        ?error,
                        query,
                        spotify_market = ?state.config.spotify_market,
                        token_source,
                        "Spotify search metadata lookup was rate-limited; stopping this import so it can be retried later"
                    );
                    return Err(error);
                }
                tracing::warn!(
                    ?error,
                    query,
                    spotify_market = ?state.config.spotify_market,
                    token_source,
                    "Spotify /v1/search lookup failed during import; skipping this row. Check status/url/body in the error."
                );
                None
            }
        };
        catalog::upsert_spotify_search_cache(&state.db, &query_key, query, track.as_ref()).await?;
        cache.insert(key, track.clone());
        return Ok(track);
    }

    Ok(None)
}

fn parse_privacy_time(value: &str, timezone: Tz) -> Result<chrono::DateTime<Utc>> {
    let parsed = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .map_err(|err| AppError::validation(format!("invalid privacy endTime {value}: {err}")))?;
    timezone
        .from_local_datetime(&parsed)
        .earliest()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| AppError::validation(format!("invalid local privacy endTime {value}")))
}

fn normalize_query_key(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn is_spotify_rate_limited(error: &AppError) -> bool {
    matches!(
        error,
        AppError::SpotifyApi { status, .. } if *status == reqwest::StatusCode::TOO_MANY_REQUESTS
    )
}

fn track_id_from_uri(uri: &str) -> Option<String> {
    uri.strip_prefix("spotify:track:")
        .or_else(|| uri.split("/track/").nth(1))
        .map(|value| value.split('?').next().unwrap_or(value).to_owned())
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_privacy_import_rows() {
        let json = br#"[{"endTime":"2024-01-01 12:34","artistName":"Artist","trackName":"Song","msPlayed":12345}]"#;
        let rows = parse_privacy_file(json, chrono_tz::UTC).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].query.as_deref(), Some("track:Song artist:Artist"));
    }

    #[test]
    fn parses_full_privacy_track_uri() {
        assert_eq!(
            track_id_from_uri("spotify:track:abc123").as_deref(),
            Some("abc123")
        );
    }
}
