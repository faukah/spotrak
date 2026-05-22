use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};

use crate::{
    domain::{
        stats::{
            AlbumReleaseYearsStats, BucketedTopAlbum, BucketedTopArtist, BucketedTopTrack,
            DiversityTimelinePoint, FeatureAverageStats, FeatureRatioStats, FeatureTimelinePoint,
            HistoryEvent, HourRepartitionPoint, HourlyTopArtist, LongestSession,
            OverviewStatsResponse, StatsDashboardBootstrapResponse, StatsDashboardResponse,
            SummaryStats, TimelinePoint, TopAlbum, TopArtist, TopTrack,
        },
        time::{IntervalQuery, RangeQuery, StatsRangeResponse, resolve_stats_range},
    },
    error::Result,
    repositories::listening_events,
    services::stats as stats_service,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(history))
        .route("/stats/range", get(stats_range))
        .route("/stats/overview", get(overview))
        .route("/stats/dashboard", get(dashboard))
        .route("/stats/dashboard/bootstrap", get(dashboard_bootstrap))
        .route("/stats/summary", get(summary))
        .route("/stats/listening-over-time", get(listening_over_time))
        .route("/stats/diversity-over-time", get(diversity_over_time))
        .route("/stats/top/tracks", get(top_tracks))
        .route("/stats/top/artists", get(top_artists))
        .route("/stats/top/albums", get(top_albums))
        .route("/stats/top/tracks-by-bucket", get(top_tracks_by_bucket))
        .route("/stats/top/artists-by-bucket", get(top_artists_by_bucket))
        .route("/stats/top/artists-by-hour", get(top_artists_by_hour))
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
        .route("/stats/feature-average", get(feature_average))
        .route(
            "/stats/feature-average-over-time",
            get(feature_average_over_time),
        )
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::history(
        &state.db,
        context.user_id,
        start,
        end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    Ok(Json(resolve_stats_range(context.timezone, query)?))
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
    let context = stats_service::private_context(&headers, &state).await?;
    Ok(Json(
        stats_service::overview_for_context(&state, &context, query).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/dashboard",
    params(IntervalQuery),
    responses((status = 200, description = "Stats dashboard data", body = StatsDashboardResponse))
)]
pub async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<StatsDashboardResponse>> {
    query.validate()?;
    let context = stats_service::private_context(&headers, &state).await?;
    Ok(Json(
        stats_service::dashboard_for_context(&state, &context, query).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/dashboard/bootstrap",
    params(IntervalQuery),
    responses((status = 200, description = "Stats dashboard bootstrap data", body = StatsDashboardBootstrapResponse))
)]
pub async fn dashboard_bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<StatsDashboardBootstrapResponse>> {
    query.validate()?;
    let context = stats_service::private_context(&headers, &state).await?;
    Ok(Json(
        stats_service::dashboard_bootstrap_for_context(&state, &context, query).await?,
    ))
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let stats = listening_events::summary(&state.db, context.user_id, start, end).await?;
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::timeline(
        &state.db,
        context.user_id,
        &context.timezone_name,
        query.split,
        start,
        end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    Ok(Json(
        listening_events::diversity_timeline(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            start,
            end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::top_tracks(
        &state.db,
        context.user_id,
        query.metric,
        start,
        end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::top_artists(
        &state.db,
        context.user_id,
        query.metric,
        start,
        end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::top_albums(
        &state.db,
        context.user_id,
        query.metric,
        start,
        end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    Ok(Json(
        listening_events::top_tracks_by_bucket(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            query.metric,
            start,
            end,
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = if query.group_other.unwrap_or(false) {
        listening_events::top_artists_by_bucket_with_other(
            &state.db,
            context.user_id,
            &context.timezone_name,
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
            context.user_id,
            &context.timezone_name,
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
    path = "/api/v1/stats/top/artists-by-hour",
    params(IntervalQuery),
    responses((status = 200, description = "Top artists by local hour", body = Vec<HourlyTopArtist>))
)]
pub async fn top_artists_by_hour(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<HourlyTopArtist>>> {
    query.validate()?;
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::top_artists_by_hour(
        &state.db,
        context.user_id,
        &context.timezone_name,
        query.metric,
        start,
        end,
        query.limit_or(1),
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    Ok(Json(
        listening_events::top_albums_by_bucket(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            query.metric,
            start,
            end,
            query.limit_or(5),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/hour-repartition/tracks",
    params(IntervalQuery),
    responses((status = 200, description = "Listening events by local hour", body = Vec<HourRepartitionPoint>))
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
    responses((status = 200, description = "Listening events by local hour; alias of the tracks endpoint", body = Vec<HourRepartitionPoint>))
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
    responses((status = 200, description = "Listening events by local hour; alias of the tracks endpoint", body = Vec<HourRepartitionPoint>))
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::hour_repartition(
        &state.db,
        context.user_id,
        &context.timezone_name,
        start,
        end,
    )
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let stats = listening_events::feature_ratio(&state.db, context.user_id, start, end).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/feature-average",
    params(IntervalQuery),
    responses((status = 200, description = "Average featured artists per listened song", body = FeatureAverageStats))
)]
pub async fn feature_average(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<FeatureAverageStats>> {
    query.validate()?;
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let stats = listening_events::feature_average(&state.db, context.user_id, start, end).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/feature-average-over-time",
    params(IntervalQuery),
    responses((status = 200, description = "Average featured artists per listened song over time", body = Vec<FeatureTimelinePoint>))
)]
pub async fn feature_average_over_time(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<Vec<FeatureTimelinePoint>>> {
    query.validate()?;
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::feature_timeline(
        &state.db,
        context.user_id,
        &context.timezone_name,
        query.split,
        start,
        end,
    )
    .await?;
    Ok(Json(rows))
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let stats =
        listening_events::album_release_years(&state.db, context.user_id, start, end).await?;
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
    let context = stats_service::private_context(&headers, &state).await?;
    let (start, end) = context.interval_bounds(&query)?;
    let rows = listening_events::longest_sessions(
        &state.db,
        context.user_id,
        start,
        end,
        query.limit_or(10),
    )
    .await?;
    Ok(Json(rows))
}
