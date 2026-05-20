use std::collections::HashSet;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::spotify::SpotifyRecentlyPlayedItem,
    error::{AppError, Result},
    repositories::catalog,
    services::spotify_client,
    state::AppState,
};

#[derive(Debug, Clone, Copy)]
pub struct IngestionResult {
    pub inserted: usize,
    pub earliest_played_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
pub fn artist_ids_from_recently_played(items: &[SpotifyRecentlyPlayedItem]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for item in items {
        for artist in &item.track.artists {
            if let Some(id) = artist.id.as_deref().filter(|id| !id.trim().is_empty()) {
                if seen.insert(id.to_owned()) {
                    ids.push(id.to_owned());
                }
            }
        }
        for artist in &item.track.album.artists {
            if let Some(id) = artist.id.as_deref().filter(|id| !id.trim().is_empty()) {
                if seen.insert(id.to_owned()) {
                    ids.push(id.to_owned());
                }
            }
        }
    }
    ids
}

pub async fn hydrate_artist_metadata(
    state: &AppState,
    access_token: &str,
    artist_ids: Vec<String>,
) -> Result<usize> {
    let mut seen = HashSet::new();
    let ids = artist_ids
        .into_iter()
        .filter(|id| !id.trim().is_empty())
        .filter(|id| seen.insert(id.clone()))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(0);
    }

    let ids = catalog::artists_missing_images(&state.db, &ids).await?;
    if ids.is_empty() {
        return Ok(0);
    }

    let mut hydrated = 0;
    let mut artists = Vec::new();
    for id in ids {
        match spotify_client::get_artist(state, access_token, &id).await {
            Ok(artist) => artists.push(artist),
            Err(error) => {
                if matches!(
                    &error,
                    AppError::SpotifyApi { status, .. }
                        if *status == reqwest::StatusCode::TOO_MANY_REQUESTS
                ) {
                    tracing::warn!(
                        ?error,
                        artist_id = %id,
                        "Spotify artist metadata hydration was rate-limited; aborting optional hydration"
                    );
                    return Err(error);
                }
                tracing::debug!(
                    ?error,
                    artist_id = %id,
                    "single Spotify /v1/artists/{id} lookup failed while hydrating artist metadata"
                );
            }
        }

        if artists.len() >= 50 {
            hydrated += artists.len();
            catalog::upsert_artist_metadata(&state.db, &artists).await?;
            artists.clear();
        }
    }
    if !artists.is_empty() {
        hydrated += artists.len();
        catalog::upsert_artist_metadata(&state.db, &artists).await?;
    }
    Ok(hydrated)
}

pub async fn ingest_recently_played(
    state: &AppState,
    user_id: Uuid,
    items: &[SpotifyRecentlyPlayedItem],
) -> Result<IngestionResult> {
    let mut tx = state.db.begin().await?;
    let mut inserted = 0;
    let mut earliest_played_at = None;

    for item in items {
        if catalog::upsert_recently_played_event(&mut tx, user_id, item, "poller", None).await? {
            inserted += 1;
        }
        earliest_played_at = Some(match earliest_played_at {
            Some(current) if current <= item.played_at => current,
            _ => item.played_at,
        });
    }

    tx.commit().await?;
    Ok(IngestionResult {
        inserted,
        earliest_played_at,
    })
}
