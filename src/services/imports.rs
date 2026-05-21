use std::collections::HashMap;

use crate::{
    domain::{
        import::{ImportFile, ImportJob},
        spotify::{SpotifyRecentlyPlayedItem, SpotifyTrack},
    },
    error::{AppError, Result},
    repositories::{catalog, imports, response_cache, spotify_queue, users},
    services::{ingestion, poller, spotify_client},
    state::AppState,
};
use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

const MIN_IMPORT_PLAY_MS: i32 = 30_000;

#[derive(Debug, Clone)]
struct ImportCandidate {
    played_at: chrono::DateTime<Utc>,
    track_id: Option<String>,
    query: Option<String>,
    track_name: Option<String>,
    artist_name: Option<String>,
}

enum LocalTrackResolution {
    Cached(Option<Box<SpotifyTrack>>),
    Uncached,
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

    let mut candidates = Vec::new();
    for file in &files {
        let bytes = tokio::fs::read(&file.path).await.map_err(|err| {
            AppError::internal(format!("failed to read {}: {err}", file.original_name))
        })?;
        let parsed = match job.import_type.as_str() {
            "privacy" => parse_privacy_file(&bytes)?,
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

    let mut cache = HashMap::<String, Option<SpotifyTrack>>::new();
    let source = match job.import_type.as_str() {
        "privacy" => "privacy-import",
        _ => "full-privacy-import",
    };
    let mut pending = Vec::<SpotifyRecentlyPlayedItem>::new();
    let mut unresolved = Vec::<ImportCandidate>::new();
    let mut current = 0_i32;
    let mut scanned = 0_i32;
    let mut locally_imported = 0_i32;
    let mut locally_skipped = 0_i32;

    for chunk in candidates.chunks(250) {
        if imports::get_any(&state.db, job.id).await?.status == "cancelled" {
            return Ok(());
        }

        for candidate in chunk {
            scanned += 1;
            if scanned % 25 == 0 && imports::get_any(&state.db, job.id).await?.status == "cancelled"
            {
                return Ok(());
            }

            if let Some(track_id) = candidate.track_id.as_deref()
                && catalog::has_user_track_play_near(
                    &state.db,
                    job.user_id,
                    track_id,
                    candidate.played_at,
                )
                .await?
            {
                current += 1;
                locally_skipped += 1;
                record_progress(state, job.id, total, current, pending.len(), cache.len()).await?;
                continue;
            }

            match resolve_cached_track(state, candidate, &mut cache).await? {
                LocalTrackResolution::Cached(Some(track)) => {
                    pending.push(SpotifyRecentlyPlayedItem {
                        track: *track,
                        played_at: candidate.played_at,
                    });
                    current += 1;
                    locally_imported += 1;
                    if pending.len() >= 250 {
                        insert_items(state, job.user_id, job.id, source, &mut pending).await?;
                    }
                    record_progress(state, job.id, total, current, pending.len(), cache.len())
                        .await?;
                }
                LocalTrackResolution::Cached(None) => {
                    current += 1;
                    locally_skipped += 1;
                    record_progress(state, job.id, total, current, pending.len(), cache.len())
                        .await?;
                }
                LocalTrackResolution::Uncached => unresolved.push(candidate.clone()),
            }
        }
    }

    insert_items(state, job.user_id, job.id, source, &mut pending).await?;
    tracing::info!(
        job_id = %job.id,
        total,
        current,
        locally_imported,
        locally_skipped,
        deferred_for_spotify = unresolved.len(),
        cache_size = cache.len(),
        "finished local import cache pass"
    );

    if !unresolved.is_empty() {
        let user = users::find_by_id(&state.db, job.user_id)
            .await?
            .ok_or(AppError::NotFound)?;
        let (mut access_token, token_source) = (
            poller::valid_access_token(state, &user).await?,
            "valid_user_token",
        );
        tracing::info!(
            job_id = %job.id,
            user_id = %user.id,
            token_source,
            spotify_market = ?state.config.spotify_market,
            remaining = unresolved.len(),
            "remaining import metadata lookups will use Spotify user access token and single-item Spotify endpoints"
        );

        for chunk in unresolved.chunks(250) {
            if imports::get_any(&state.db, job.id).await?.status == "cancelled" {
                return Ok(());
            }

            for candidate in chunk {
                if current % 25 == 0
                    && imports::get_any(&state.db, job.id).await?.status == "cancelled"
                {
                    return Ok(());
                }

                if let Some(track_id) = candidate.track_id.as_deref()
                    && catalog::has_user_track_play_near(
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

                if current == 0 || current % 100 == 0 {
                    tracing::debug!(
                        job_id = %job.id,
                        next = current + 1,
                        total,
                        cache_size = cache.len(),
                        "resolving import metadata via Spotify"
                    );
                }

                if let Some(track) = resolve_track(
                    state,
                    job.user_id,
                    &mut access_token,
                    token_source,
                    candidate,
                    &mut cache,
                )
                .await?
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
    }

    imports::mark_success(&state.db, job.id, total, current).await?;
    if let Err(error) = cleanup_import_files(state, job.id, &files).await {
        tracing::warn!(?error, job_id = %job.id, "failed to clean up raw import files after successful import");
    }

    Ok(())
}

async fn cleanup_import_files(
    state: &AppState,
    job_id: uuid::Uuid,
    files: &[ImportFile],
) -> Result<()> {
    for file in files {
        match tokio::fs::remove_file(&file.path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(?error, path = %file.path, "failed to delete raw import file after successful import");
            }
        }
    }
    let job_dir = state.config.import_dir.join(job_id.to_string());
    match tokio::fs::remove_dir_all(&job_dir).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::debug!(?error, path = %job_dir.display(), "failed to delete import directory after successful import");
        }
    }
    imports::delete_files(&state.db, job_id).await?;
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
        response_cache::invalidate_stats(&state.db, user_id).await?;
        let missing = catalog::artists_missing_images(&state.db, &artist_ids).await?;
        spotify_queue::enqueue_artist_hydration(&state.db, user_id, &missing).await?;
    }

    Ok(())
}

fn parse_privacy_file(bytes: &[u8]) -> Result<Vec<ImportCandidate>> {
    let entries = serde_json::from_slice::<Vec<PrivacyEntry>>(bytes)
        .map_err(|err| AppError::validation(format!("invalid privacy JSON: {err}")))?;
    let mut candidates = Vec::new();
    for entry in entries {
        if entry.ms_played.unwrap_or_default() < MIN_IMPORT_PLAY_MS {
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
        let played_at = parse_privacy_time(&end_time)?;
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
        if entry.ms_played.unwrap_or_default() < MIN_IMPORT_PLAY_MS {
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

async fn resolve_cached_track(
    state: &AppState,
    candidate: &ImportCandidate,
    cache: &mut HashMap<String, Option<SpotifyTrack>>,
) -> Result<LocalTrackResolution> {
    if let Some(track_id) = &candidate.track_id {
        let key = format!("id:{track_id}");
        if let Some(cached) = cache.get(&key) {
            return Ok(LocalTrackResolution::Cached(cached.clone().map(Box::new)));
        }
        if let Some(track) = catalog::spotify_track_from_cache(&state.db, track_id).await? {
            tracing::debug!(
                track_id,
                "using cached Spotify track metadata from database"
            );
            cache.insert(key, Some(track.clone()));
            return Ok(LocalTrackResolution::Cached(Some(Box::new(track))));
        }
        return Ok(LocalTrackResolution::Uncached);
    }

    if let Some(query) = &candidate.query {
        let key = format!("q:{query}");
        if let Some(cached) = cache.get(&key) {
            return Ok(LocalTrackResolution::Cached(cached.clone().map(Box::new)));
        }
        if let (Some(track_name), Some(artist_name)) =
            (&candidate.track_name, &candidate.artist_name)
            && let Some(track) =
                catalog::spotify_track_from_name_artist_cache(&state.db, track_name, artist_name)
                    .await?
        {
            tracing::debug!(
                track_name,
                artist_name,
                "using cached Spotify search result from database"
            );
            cache.insert(key, Some(track.clone()));
            return Ok(LocalTrackResolution::Cached(Some(Box::new(track))));
        }
        let query_key = normalize_query_key(query);
        if let Some(hit) = catalog::spotify_search_cache(&state.db, &query_key).await? {
            match hit {
                catalog::SpotifySearchCacheHit::Found(track) => {
                    let track = (*track).clone();
                    tracing::debug!(query, "using cached Spotify search result");
                    cache.insert(key, Some(track.clone()));
                    return Ok(LocalTrackResolution::Cached(Some(Box::new(track))));
                }
                catalog::SpotifySearchCacheHit::NotFound => {
                    tracing::debug!(query, "using cached negative Spotify search result");
                    cache.insert(key, None);
                    return Ok(LocalTrackResolution::Cached(None));
                }
            }
        }
    }

    Ok(LocalTrackResolution::Uncached)
}

async fn resolve_track(
    state: &AppState,
    user_id: uuid::Uuid,
    access_token: &mut String,
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
        let track = match get_track_with_reauth(state, user_id, access_token, track_id).await {
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
                if poller::is_spotify_authorization_error(&error) {
                    tracing::warn!(
                        ?error,
                        track_id,
                        spotify_market = ?state.config.spotify_market,
                        token_source,
                        "Spotify track metadata lookup still failed after token refresh; stopping this import until the user reconnects Spotify"
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
            && let Some(track) =
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
        let query_key = normalize_query_key(query);
        if let Some(hit) = catalog::spotify_search_cache(&state.db, &query_key).await? {
            match hit {
                catalog::SpotifySearchCacheHit::Found(track) => {
                    let track = (*track).clone();
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
        let track = match search_track_with_reauth(state, user_id, access_token, query).await {
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
                if poller::is_spotify_authorization_error(&error) {
                    tracing::warn!(
                        ?error,
                        query,
                        spotify_market = ?state.config.spotify_market,
                        token_source,
                        "Spotify search metadata lookup still failed after token refresh; stopping this import until the user reconnects Spotify"
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

async fn get_track_with_reauth(
    state: &AppState,
    user_id: uuid::Uuid,
    access_token: &mut String,
    track_id: &str,
) -> Result<SpotifyTrack> {
    match spotify_client::get_track(state, access_token.as_str(), track_id).await {
        Err(error) if poller::is_spotify_authorization_error(&error) => {
            tracing::info!(
                user_id = %user_id,
                track_id,
                "refreshing Spotify token after track lookup authorization failure"
            );
            *access_token = poller::force_refresh_access_token(state, user_id).await?;
            spotify_client::get_track(state, access_token.as_str(), track_id).await
        }
        result => result,
    }
}

async fn search_track_with_reauth(
    state: &AppState,
    user_id: uuid::Uuid,
    access_token: &mut String,
    query: &str,
) -> Result<Option<SpotifyTrack>> {
    match spotify_client::search_track(state, access_token.as_str(), query).await {
        Err(error) if poller::is_spotify_authorization_error(&error) => {
            tracing::info!(
                user_id = %user_id,
                query,
                "refreshing Spotify token after search authorization failure"
            );
            *access_token = poller::force_refresh_access_token(state, user_id).await?;
            spotify_client::search_track(state, access_token.as_str(), query).await
        }
        result => result,
    }
}

fn parse_privacy_time(value: &str) -> Result<chrono::DateTime<Utc>> {
    let parsed = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .map_err(|err| AppError::validation(format!("invalid privacy endTime {value}: {err}")))?;
    Ok(Utc.from_utc_datetime(&parsed))
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
        .map(|value| value.split('?').next().unwrap_or(value))
        .filter(|value| is_spotify_id(value))
        .map(ToOwned::to_owned)
}

fn is_spotify_id(value: &str) -> bool {
    value.len() == 22 && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_privacy_import_rows() {
        let json = br#"[{"endTime":"2024-01-01 12:34","artistName":"Artist","trackName":"Song","msPlayed":30000}]"#;
        let rows = parse_privacy_file(json).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].query.as_deref(), Some("track:Song artist:Artist"));
    }

    #[test]
    fn skips_short_import_rows() {
        let privacy = br#"[{"endTime":"2024-01-01 12:34","artistName":"Artist","trackName":"Song","msPlayed":29999}]"#;
        assert!(parse_privacy_file(privacy).unwrap().is_empty());

        let full_privacy = br#"[{"ts":"2024-01-01T12:34:00Z","ms_played":29999,"master_metadata_track_name":"Song","master_metadata_album_artist_name":"Artist","spotify_track_uri":"spotify:track:abc123"}]"#;
        assert!(parse_full_privacy_file(full_privacy).unwrap().is_empty());
    }

    #[test]
    fn parses_privacy_import_timestamps_as_utc() {
        let rows = parse_privacy_file(
            br#"[{"endTime":"2024-01-01 12:34","artistName":"Artist","trackName":"Song","msPlayed":30000}]"#,
        )
        .unwrap();
        assert_eq!(rows[0].played_at.to_rfc3339(), "2024-01-01T12:34:00+00:00");
    }

    #[test]
    fn parses_full_privacy_track_uri() {
        assert_eq!(
            track_id_from_uri("spotify:track:6KzkqZqhUBEsWYJJa2aBOd").as_deref(),
            Some("6KzkqZqhUBEsWYJJa2aBOd")
        );
    }

    #[test]
    fn rejects_invalid_full_privacy_track_uri() {
        assert_eq!(track_id_from_uri("spotify:track:abc/def"), None);
        assert_eq!(
            track_id_from_uri("https://open.spotify.com/track/6KzkqZqhUBEsWYJJa2aBOd/extra"),
            None
        );
    }
}
