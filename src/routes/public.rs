use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use uuid::Uuid;

use crate::{
    domain::{
        catalog::{AlbumDetail, ArtistDetail, TrackDetail},
        stats::{
            DiversityTimelinePoint, EntityStats, HistoryEvent, SummaryStats, TimelinePoint,
            TopAlbum, TopArtist, TopTrack,
        },
        time::IntervalQuery,
    },
    error::{AppError, Result},
    repositories::{catalog, listening_events, public_tokens, settings},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/public/{token}/history", get(history))
        .route("/public/{token}/stats/summary", get(summary))
        .route(
            "/public/{token}/stats/listening-over-time",
            get(listening_over_time),
        )
        .route(
            "/public/{token}/stats/diversity-over-time",
            get(diversity_over_time),
        )
        .route("/public/{token}/stats/top/tracks", get(top_tracks))
        .route("/public/{token}/stats/top/artists", get(top_artists))
        .route("/public/{token}/stats/top/albums", get(top_albums))
        .route("/public/{token}/tracks/{id}", get(track))
        .route("/public/{token}/tracks/{id}/stats", get(track_stats))
        .route("/public/{token}/artists/{id}", get(artist))
        .route("/public/{token}/artists/{id}/stats", get(artist_stats))
        .route("/public/{token}/albums/{id}", get(album))
        .route("/public/{token}/albums/{id}/stats", get(album_stats))
}

async fn user_id_for_token(state: &AppState, token: &str) -> Result<Uuid> {
    public_tokens::user_id_for_token(&state.db, token)
        .await?
        .ok_or(AppError::Unauthorized)
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/history",
    params(IntervalQuery),
    responses((status = 200, description = "Public listening history", body = Vec<HistoryEvent>))
)]
pub async fn history(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HistoryEvent>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::history(
            &state.db,
            user_id,
            query.start,
            query.end,
            query.limit_or(50),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/summary",
    params(IntervalQuery),
    responses((status = 200, description = "Public summary stats", body = SummaryStats))
)]
pub async fn summary(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<SummaryStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::summary(&state.db, user_id, query.start, query.end).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/listening-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Public listening over time", body = Vec<TimelinePoint>))
)]
pub async fn listening_over_time(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TimelinePoint>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let user_settings = settings::get(&state.db, user_id).await?;
    let timezone = user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned());
    Ok(Json(
        listening_events::timeline(
            &state.db,
            user_id,
            &timezone,
            query.split,
            query.start,
            query.end,
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/top/tracks",
    params(IntervalQuery),
    responses((status = 200, description = "Public top tracks", body = Vec<TopTrack>))
)]
pub async fn top_tracks(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopTrack>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::top_tracks(
            &state.db,
            user_id,
            query.metric,
            query.start,
            query.end,
            query.limit_or(20),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/top/artists",
    params(IntervalQuery),
    responses((status = 200, description = "Public top artists", body = Vec<TopArtist>))
)]
pub async fn top_artists(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopArtist>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::top_artists(
            &state.db,
            user_id,
            query.metric,
            query.start,
            query.end,
            query.limit_or(20),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/top/albums",
    params(IntervalQuery),
    responses((status = 200, description = "Public top albums", body = Vec<TopAlbum>))
)]
pub async fn top_albums(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopAlbum>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::top_albums(
            &state.db,
            user_id,
            query.metric,
            query.start,
            query.end,
            query.limit_or(20),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/tracks/{id}",
    responses((status = 200, description = "Public track detail", body = TrackDetail))
)]
pub async fn track(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
) -> Result<Json<TrackDetail>> {
    let _ = user_id_for_token(&state, &token).await?;
    Ok(Json(catalog::track(&state.db, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/artists/{id}",
    responses((status = 200, description = "Public artist detail", body = ArtistDetail))
)]
pub async fn artist(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
) -> Result<Json<ArtistDetail>> {
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(catalog::artist(&state.db, user_id, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/albums/{id}",
    responses((status = 200, description = "Public album detail", body = AlbumDetail))
)]
pub async fn album(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
) -> Result<Json<AlbumDetail>> {
    let _ = user_id_for_token(&state, &token).await?;
    Ok(Json(catalog::album(&state.db, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/tracks/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Public track stats", body = EntityStats))
)]
pub async fn track_stats(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Track(&id),
            query.start,
            query.end,
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/artists/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Public artist stats", body = EntityStats))
)]
pub async fn artist_stats(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Artist(&id),
            query.start,
            query.end,
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/albums/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Public album stats", body = EntityStats))
)]
pub async fn album_stats(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, String)>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Album(&id),
            query.start,
            query.end,
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/diversity-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Public diversity over time", body = Vec<DiversityTimelinePoint>))
)]
pub async fn diversity_over_time(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<DiversityTimelinePoint>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let user_settings = settings::get(&state.db, user_id).await?;
    let timezone = user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned());
    Ok(Json(
        listening_events::diversity_timeline(
            &state.db,
            user_id,
            &timezone,
            query.split,
            query.start,
            query.end,
        )
        .await?,
    ))
}
