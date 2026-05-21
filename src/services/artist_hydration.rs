use std::collections::{HashMap, HashSet};

use crate::{
    error::Result,
    repositories::{catalog, spotify_queue, users},
    services::{ingestion, poller},
    state::AppState,
};

const ARTIST_HYDRATION_BATCH_SIZE: i64 = 3;

pub async fn process_queued_once(state: &AppState) -> Result<usize> {
    let jobs =
        spotify_queue::claim_artist_hydration_jobs(&state.db, ARTIST_HYDRATION_BATCH_SIZE).await?;
    let mut hydrated = 0;
    let mut jobs_by_user = HashMap::new();

    for job in jobs {
        jobs_by_user
            .entry(job.user_id)
            .or_insert_with(Vec::new)
            .push(job);
    }

    for (user_id, jobs) in jobs_by_user {
        let requested = jobs
            .iter()
            .map(|job| job.artist_id.clone())
            .collect::<Vec<_>>();
        let missing = catalog::artists_missing_images(&state.db, &requested).await?;
        let missing_set = missing.iter().cloned().collect::<HashSet<_>>();
        let mut missing_jobs = Vec::new();

        for job in &jobs {
            if missing_set.contains(&job.artist_id) {
                missing_jobs.push(job);
            } else {
                spotify_queue::complete_artist_hydration(&state.db, &job.artist_id).await?;
            }
        }

        if missing_jobs.is_empty() {
            continue;
        }

        let Some(user) = users::find_by_id(&state.db, user_id).await? else {
            complete_jobs(state, &missing_jobs).await?;
            continue;
        };

        let mut access_token = match poller::valid_access_token(state, &user).await {
            Ok(access_token) => access_token,
            Err(error) => {
                let message = error.to_string();
                fail_jobs(state, &missing_jobs, &message).await?;
                tracing::debug!(
                    ?error,
                    user_id = %user_id,
                    artist_count = missing_jobs.len(),
                    "artist metadata hydration postponed because no Spotify token is available"
                );
                continue;
            }
        };

        let hydration =
            match ingestion::hydrate_artist_metadata(state, &access_token, missing.clone()).await {
                Err(error) if poller::is_spotify_authorization_error(&error) => {
                    tracing::info!(
                        user_id = %user.id,
                        artist_count = missing_jobs.len(),
                        "refreshing Spotify token after artist hydration authorization failure"
                    );
                    access_token = poller::force_refresh_access_token(state, user.id).await?;
                    ingestion::hydrate_artist_metadata(state, &access_token, missing).await
                }
                result => result,
            };

        match hydration {
            Ok(count) => {
                hydrated += count;
                complete_jobs(state, &missing_jobs).await?;
            }
            Err(error) => {
                let message = error.to_string();
                fail_jobs(state, &missing_jobs, &message).await?;
                tracing::debug!(
                    ?error,
                    user_id = %user_id,
                    artist_count = missing_jobs.len(),
                    "artist metadata hydration failed and will be retried"
                );
            }
        }
    }

    Ok(hydrated)
}

async fn complete_jobs(
    state: &AppState,
    jobs: &[&spotify_queue::ArtistHydrationJob],
) -> Result<()> {
    for job in jobs {
        spotify_queue::complete_artist_hydration(&state.db, &job.artist_id).await?;
    }
    Ok(())
}

async fn fail_jobs(
    state: &AppState,
    jobs: &[&spotify_queue::ArtistHydrationJob],
    error: &str,
) -> Result<()> {
    for job in jobs {
        spotify_queue::fail_artist_hydration(
            &state.db,
            &job.artist_id,
            job.user_id,
            job.attempts,
            error,
        )
        .await?;
    }
    Ok(())
}
