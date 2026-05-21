use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{
    domain::{
        catalog::{AlbumDetail, ArtistDetail, TrackDetail},
        settings::StatsDisplayPreferences,
        stats::{
            AlbumReleaseYearsStats, BucketedTopArtist, DiversityTimelinePoint, EntityStats,
            FeatureAverageStats, FeatureTimelinePoint, HistoryEvent, HourRepartitionPoint,
            HourlyTopArtist, StatsDashboardResponse, SummaryStats, TimelinePoint, TopAlbum,
            TopArtist, TopTrack,
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
        .route(
            "/public/{token}/stats/display-preferences",
            get(display_preferences),
        )
        .route("/public/{token}/stats/summary", get(summary))
        .route("/public/{token}/stats/dashboard", get(dashboard))
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
        .route(
            "/public/{token}/stats/top/artists-by-bucket",
            get(top_artists_by_bucket),
        )
        .route(
            "/public/{token}/stats/top/artists-by-hour",
            get(top_artists_by_hour),
        )
        .route("/public/{token}/stats/top/albums", get(top_albums))
        .route(
            "/public/{token}/stats/hour-repartition/tracks",
            get(hour_repartition_tracks),
        )
        .route(
            "/public/{token}/stats/feature-average",
            get(feature_average),
        )
        .route(
            "/public/{token}/stats/feature-average-over-time",
            get(feature_average_over_time),
        )
        .route(
            "/public/{token}/stats/album-release-years",
            get(album_release_years),
        )
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

async fn ensure_public_track(state: &AppState, user_id: Uuid, id: &str) -> Result<()> {
    if catalog::user_has_track(&state.db, user_id, id).await? {
        return Ok(());
    }
    Err(AppError::NotFound)
}

async fn ensure_public_artist(state: &AppState, user_id: Uuid, id: &str) -> Result<()> {
    if catalog::user_has_artist(&state.db, user_id, id).await? {
        return Ok(());
    }
    Err(AppError::NotFound)
}

async fn ensure_public_album(state: &AppState, user_id: Uuid, id: &str) -> Result<()> {
    if catalog::user_has_album(&state.db, user_id, id).await? {
        return Ok(());
    }
    Err(AppError::NotFound)
}

fn parse_timezone(value: &str) -> Result<Tz> {
    value
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))
}

async fn interval_bounds(
    state: &AppState,
    user_id: Uuid,
    query: &IntervalQuery,
) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    if query.range.is_none() {
        return Ok((query.start, query.end));
    }
    let timezone = user_timezone(state, user_id).await?;
    query.resolved_bounds(parse_timezone(&timezone)?)
}

async fn user_timezone(state: &AppState, user_id: Uuid) -> Result<String> {
    let user_settings = settings::get(&state.db, user_id).await?;
    Ok(user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned()))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/display-preferences",
    responses((status = 200, description = "Public stats display preferences", body = StatsDisplayPreferences))
)]
pub async fn display_preferences(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<StatsDisplayPreferences>> {
    let user_id = user_id_for_token(&state, &token).await?;
    let user_settings = settings::get(&state.db, user_id).await?;
    Ok(Json(StatsDisplayPreferences {
        hour_format: user_settings.hour_format,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/dashboard",
    params(IntervalQuery),
    responses((status = 200, description = "Public stats dashboard data", body = StatsDashboardResponse))
)]
pub async fn dashboard(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<StatsDashboardResponse>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    Ok(Json(
        crate::routes::stats::dashboard_for_user(&state, user_id, query).await?,
    ))
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
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::history(
            &state.db,
            user_id,
            start,
            end,
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
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::summary(&state.db, user_id, start, end).await?,
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
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::timeline(&state.db, user_id, &timezone, query.split, start, end).await?,
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
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_tracks(
            &state.db,
            user_id,
            query.metric,
            start,
            end,
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
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_artists(
            &state.db,
            user_id,
            query.metric,
            start,
            end,
            query.limit_or(20),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/top/artists-by-bucket",
    params(IntervalQuery),
    responses((status = 200, description = "Public top artists by time bucket", body = Vec<BucketedTopArtist>))
)]
pub async fn top_artists_by_bucket(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<BucketedTopArtist>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = if query.group_other.unwrap_or(false) {
        listening_events::top_artists_by_bucket_with_other(
            &state.db,
            user_id,
            &timezone,
            query.split,
            query.metric,
            start,
            end,
            query.limit_or(10),
        )
        .await?
    } else {
        listening_events::top_artists_by_bucket(
            &state.db,
            user_id,
            &timezone,
            query.split,
            query.metric,
            start,
            end,
            query.limit_or(5),
        )
        .await?
    };
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/top/artists-by-hour",
    params(IntervalQuery),
    responses((status = 200, description = "Public top artists by local hour", body = Vec<HourlyTopArtist>))
)]
pub async fn top_artists_by_hour(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourlyTopArtist>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_artists_by_hour(
            &state.db,
            user_id,
            &timezone,
            query.metric,
            start,
            end,
            query.limit_or(1),
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
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_albums(
            &state.db,
            user_id,
            query.metric,
            start,
            end,
            query.limit_or(20),
            query.offset_or_zero(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/hour-repartition/tracks",
    params(IntervalQuery),
    responses((status = 200, description = "Public track plays by local hour", body = Vec<HourRepartitionPoint>))
)]
pub async fn hour_repartition_tracks(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourRepartitionPoint>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::hour_repartition(&state.db, user_id, &timezone, start, end).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/feature-average",
    params(IntervalQuery),
    responses((status = 200, description = "Public average featured artists per listened song", body = FeatureAverageStats))
)]
pub async fn feature_average(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<FeatureAverageStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::feature_average(&state.db, user_id, start, end).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/feature-average-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Public average featured artists per listened song over time", body = Vec<FeatureTimelinePoint>))
)]
pub async fn feature_average_over_time(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<FeatureTimelinePoint>>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::feature_timeline(&state.db, user_id, &timezone, query.split, start, end)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/public/{token}/stats/album-release-years",
    params(IntervalQuery),
    responses((status = 200, description = "Public album release-year distribution", body = AlbumReleaseYearsStats))
)]
pub async fn album_release_years(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<AlbumReleaseYearsStats>> {
    query.validate()?;
    let user_id = user_id_for_token(&state, &token).await?;
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::album_release_years(&state.db, user_id, start, end).await?,
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
    let user_id = user_id_for_token(&state, &token).await?;
    ensure_public_track(&state, user_id, &id).await?;
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
    ensure_public_artist(&state, user_id, &id).await?;
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
    let user_id = user_id_for_token(&state, &token).await?;
    ensure_public_album(&state, user_id, &id).await?;
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
    ensure_public_track(&state, user_id, &id).await?;
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Track(&id),
            start,
            end,
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
    ensure_public_artist(&state, user_id, &id).await?;
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Artist(&id),
            start,
            end,
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
    ensure_public_album(&state, user_id, &id).await?;
    let (start, end) = interval_bounds(&state, user_id, &query).await?;
    Ok(Json(
        listening_events::entity_stats(
            &state.db,
            user_id,
            listening_events::EntityFilter::Album(&id),
            start,
            end,
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
    let timezone = user_timezone(&state, user_id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::diversity_timeline(
            &state.db,
            user_id,
            &timezone,
            query.split,
            start,
            end,
        )
        .await?,
    ))
}
