use std::collections::BTreeSet;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{Datelike, Utc};
use chrono_tz::Tz;

use crate::{
    auth::extractors::current_user,
    domain::{
        stats::{
            AlbumReleaseYearsStats, BucketedTopAlbum, BucketedTopArtist, BucketedTopTrack,
            DiversityTimelinePoint, FeatureRatioStats, HistoryEvent, HourRepartitionPoint,
            LongestSession, OverviewStatsResponse, SummaryStats, TimelinePoint, TopAlbum,
            TopArtist, TopTrack,
        },
        time::{
            IntervalQuery, Metric, RangeQuery, StatsRangeResponse, TimeSplit, resolve_stats_range,
        },
    },
    error::{AppError, Result},
    repositories::{listening_events, settings},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(history))
        .route("/stats/range", get(stats_range))
        .route("/stats/overview", get(overview))
        .route("/stats/summary", get(summary))
        .route("/stats/listening-over-time", get(listening_over_time))
        .route("/stats/diversity-over-time", get(diversity_over_time))
        .route("/stats/top/tracks", get(top_tracks))
        .route("/stats/top/artists", get(top_artists))
        .route("/stats/top/albums", get(top_albums))
        .route("/stats/top/tracks-by-bucket", get(top_tracks_by_bucket))
        .route("/stats/top/artists-by-bucket", get(top_artists_by_bucket))
        .route("/stats/top/albums-by-bucket", get(top_albums_by_bucket))
        .route(
            "/stats/hour-repartition/tracks",
            get(hour_repartition_tracks),
        )
        .route(
            "/stats/hour-repartition/artists",
            get(hour_repartition_artists),
        )
        .route(
            "/stats/hour-repartition/albums",
            get(hour_repartition_albums),
        )
        .route("/stats/feature-ratio", get(feature_ratio))
        .route("/stats/album-release-years", get(album_release_years))
        .route("/stats/longest-sessions", get(longest_sessions))
}

#[utoipa::path(
    get,
    path = "/api/v1/history",
    params(IntervalQuery),
    responses((status = 200, description = "Listening history", body = Vec<HistoryEvent>))
)]
pub async fn history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HistoryEvent>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let rows = listening_events::history(
        &state.db,
        user.id,
        query.start,
        query.end,
        query.limit_or(50),
        query.offset_or_zero(),
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/range",
    params(RangeQuery),
    responses((status = 200, description = "Resolved stats range", body = StatsRangeResponse))
)]
pub async fn stats_range(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RangeQuery>,
) -> Result<Json<StatsRangeResponse>> {
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let timezone = timezone
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))?;
    Ok(Json(resolve_stats_range(timezone, query)?))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/overview",
    params(RangeQuery),
    responses((status = 200, description = "Overview dashboard data", body = OverviewStatsResponse))
)]
pub async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RangeQuery>,
) -> Result<Json<OverviewStatsResponse>> {
    let user = current_user(&headers, &state).await?;
    let timezone_name = user_timezone(&state, user.id).await?;
    let timezone = timezone_name
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))?;
    let range = resolve_stats_range(timezone, query)?;

    let summary = listening_events::summary(&state.db, user.id, range.start, range.end).await?;
    let previous_summary = match (range.previous_start, range.previous_end) {
        (Some(start), Some(end)) => {
            Some(listening_events::summary(&state.db, user.id, Some(start), Some(end)).await?)
        }
        _ => None,
    };

    let mut artists = listening_events::top_artists(
        &state.db,
        user.id,
        Metric::Count,
        range.start,
        range.end,
        1,
        0,
    )
    .await?;
    let best_artist = artists.pop();
    let best_artist_stats = match &best_artist {
        Some(artist) => Some(
            listening_events::entity_stats(
                &state.db,
                user.id,
                listening_events::EntityFilter::Artist(&artist.id),
                range.start,
                range.end,
            )
            .await?,
        ),
        None => None,
    };

    let mut tracks = listening_events::top_tracks(
        &state.db,
        user.id,
        Metric::Count,
        range.start,
        range.end,
        1,
        0,
    )
    .await?;
    let best_song = tracks.pop();

    let hourly_distribution = listening_events::hour_repartition(
        &state.db,
        user.id,
        &timezone_name,
        range.start,
        range.end,
    )
    .await?;
    let history =
        listening_events::history(&state.db, user.id, range.start, range.end, 25, 0).await?;
    let available_years = available_years(&state, user.id, timezone, &timezone_name).await?;

    Ok(Json(OverviewStatsResponse {
        range,
        available_years,
        summary,
        previous_summary,
        best_artist,
        best_artist_stats,
        best_song,
        hourly_distribution,
        history,
    }))
}

async fn available_years(
    state: &AppState,
    user_id: uuid::Uuid,
    timezone: Tz,
    timezone_name: &str,
) -> Result<Vec<i32>> {
    let now_year = Utc::now().with_timezone(&timezone).year();
    let timeline = listening_events::timeline(
        &state.db,
        user_id,
        timezone_name,
        TimeSplit::Year,
        None,
        None,
    )
    .await?;
    let mut years = BTreeSet::new();
    years.insert(now_year);
    for point in timeline {
        if let Some(year) = point
            .bucket
            .get(0..4)
            .and_then(|value| value.parse::<i32>().ok())
        {
            years.insert(year);
        }
    }
    Ok(years.into_iter().rev().collect())
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/summary",
    params(IntervalQuery),
    responses((status = 200, description = "Summary stats", body = SummaryStats))
)]
pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<SummaryStats>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let stats = listening_events::summary(&state.db, user.id, query.start, query.end).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/listening-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Listening over time", body = Vec<TimelinePoint>))
)]
pub async fn listening_over_time(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TimelinePoint>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let rows = listening_events::timeline(
        &state.db,
        user.id,
        &timezone,
        query.split,
        query.start,
        query.end,
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/diversity-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Listening diversity over time", body = Vec<DiversityTimelinePoint>))
)]
pub async fn diversity_over_time(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<DiversityTimelinePoint>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    Ok(Json(
        listening_events::diversity_timeline(
            &state.db,
            user.id,
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
    path = "/api/v1/stats/top/tracks",
    params(IntervalQuery),
    responses((status = 200, description = "Top tracks", body = Vec<TopTrack>))
)]
pub async fn top_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopTrack>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let rows = listening_events::top_tracks(
        &state.db,
        user.id,
        query.metric,
        query.start,
        query.end,
        query.limit_or(20),
        query.offset_or_zero(),
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/top/artists",
    params(IntervalQuery),
    responses((status = 200, description = "Top artists", body = Vec<TopArtist>))
)]
pub async fn top_artists(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopArtist>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let rows = listening_events::top_artists(
        &state.db,
        user.id,
        query.metric,
        query.start,
        query.end,
        query.limit_or(20),
        query.offset_or_zero(),
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/top/albums",
    params(IntervalQuery),
    responses((status = 200, description = "Top albums", body = Vec<TopAlbum>))
)]
pub async fn top_albums(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<TopAlbum>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let rows = listening_events::top_albums(
        &state.db,
        user.id,
        query.metric,
        query.start,
        query.end,
        query.limit_or(20),
        query.offset_or_zero(),
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/top/tracks-by-bucket",
    params(IntervalQuery),
    responses((status = 200, description = "Top tracks by time bucket", body = Vec<BucketedTopTrack>))
)]
pub async fn top_tracks_by_bucket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<BucketedTopTrack>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    Ok(Json(
        listening_events::top_tracks_by_bucket(
            &state.db,
            user.id,
            &timezone,
            query.split,
            query.metric,
            query.start,
            query.end,
            query.limit_or(5),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/top/artists-by-bucket",
    params(IntervalQuery),
    responses((status = 200, description = "Top artists by time bucket", body = Vec<BucketedTopArtist>))
)]
pub async fn top_artists_by_bucket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<BucketedTopArtist>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let rows = listening_events::top_artists_by_bucket(
        &state.db,
        user.id,
        &timezone,
        query.split,
        query.metric,
        query.start,
        query.end,
        query.limit_or(5),
    )
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/top/albums-by-bucket",
    params(IntervalQuery),
    responses((status = 200, description = "Top albums by time bucket", body = Vec<BucketedTopAlbum>))
)]
pub async fn top_albums_by_bucket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<BucketedTopAlbum>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    Ok(Json(
        listening_events::top_albums_by_bucket(
            &state.db,
            user.id,
            &timezone,
            query.split,
            query.metric,
            query.start,
            query.end,
            query.limit_or(5),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/hour-repartition/tracks",
    params(IntervalQuery),
    responses((status = 200, description = "Track plays by local hour", body = Vec<HourRepartitionPoint>))
)]
pub async fn hour_repartition_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourRepartitionPoint>>> {
    hour_repartition(state, headers, query).await
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/hour-repartition/artists",
    params(IntervalQuery),
    responses((status = 200, description = "Artist plays by local hour", body = Vec<HourRepartitionPoint>))
)]
pub async fn hour_repartition_artists(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourRepartitionPoint>>> {
    hour_repartition(state, headers, query).await
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/hour-repartition/albums",
    params(IntervalQuery),
    responses((status = 200, description = "Album plays by local hour", body = Vec<HourRepartitionPoint>))
)]
pub async fn hour_repartition_albums(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourRepartitionPoint>>> {
    hour_repartition(state, headers, query).await
}

async fn hour_repartition(
    state: AppState,
    headers: HeaderMap,
    query: IntervalQuery,
) -> Result<Json<Vec<HourRepartitionPoint>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let rows =
        listening_events::hour_repartition(&state.db, user.id, &timezone, query.start, query.end)
            .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/feature-ratio",
    params(IntervalQuery),
    responses((status = 200, description = "Solo vs multi-artist play ratio", body = FeatureRatioStats))
)]
pub async fn feature_ratio(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<FeatureRatioStats>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let stats = listening_events::feature_ratio(&state.db, user.id, query.start, query.end).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/album-release-years",
    params(IntervalQuery),
    responses((status = 200, description = "Album release-year distribution", body = AlbumReleaseYearsStats))
)]
pub async fn album_release_years(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<AlbumReleaseYearsStats>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let stats =
        listening_events::album_release_years(&state.db, user.id, query.start, query.end).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/longest-sessions",
    params(IntervalQuery),
    responses((status = 200, description = "Longest listening sessions", body = Vec<LongestSession>))
)]
pub async fn longest_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<LongestSession>>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    let rows = listening_events::longest_sessions(
        &state.db,
        user.id,
        query.start,
        query.end,
        query.limit_or(10),
    )
    .await?;
    Ok(Json(rows))
}

async fn user_timezone(state: &AppState, user_id: uuid::Uuid) -> Result<String> {
    let user_settings = settings::get(&state.db, user_id).await?;
    Ok(user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned()))
}
