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

    for job in jobs {
        let requested = vec![job.artist_id.clone()];
        let missing = catalog::artists_missing_images(&state.db, &requested).await?;
        if missing.is_empty() {
            spotify_queue::complete_artist_hydration(&state.db, &job.artist_id).await?;
            continue;
        }

        let Some(user) = users::find_by_id(&state.db, job.user_id).await? else {
            spotify_queue::complete_artist_hydration(&state.db, &job.artist_id).await?;
            continue;
        };

        let mut access_token = match poller::valid_access_token(state, &user).await {
            Ok(access_token) => access_token,
            Err(error) => {
                spotify_queue::fail_artist_hydration(
                    &state.db,
                    &job.artist_id,
                    job.attempts,
                    &error.to_string(),
                )
                .await?;
                tracing::debug!(
                    ?error,
                    artist_id = %job.artist_id,
                    "artist metadata hydration postponed because no Spotify token is available"
                );
                continue;
            }
        };

        let hydration =
            match ingestion::hydrate_artist_metadata(state, &access_token, missing.clone()).await {
                Err(error) if poller::is_spotify_authorization_error(&error) => {
                    tracing::info!(
                        artist_id = %job.artist_id,
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
                spotify_queue::complete_artist_hydration(&state.db, &job.artist_id).await?;
            }
            Err(error) => {
                spotify_queue::fail_artist_hydration(
                    &state.db,
                    &job.artist_id,
                    job.attempts,
                    &error.to_string(),
                )
                .await?;
                tracing::debug!(
                    ?error,
                    artist_id = %job.artist_id,
                    "artist metadata hydration failed and will be retried"
                );
            }
        }
    }

    Ok(hydrated)
}
