use std::collections::BTreeSet;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use chrono_tz::Tz;

use crate::{
    auth::extractors::current_user,
    domain::{
        stats::{
            AlbumReleaseYearsStats, BucketedTopAlbum, BucketedTopArtist, BucketedTopTrack,
            DiversityTimelinePoint, FeatureAverageStats, FeatureRatioStats, FeatureTimelinePoint,
            HistoryEvent, HourRepartitionPoint, HourlyTopArtist, LongestSession,
            OverviewStatsResponse, StatsDashboardResponse, SummaryStats, TimelinePoint, TopAlbum,
            TopArtist, TopTrack,
        },
        time::{
            IntervalQuery, Metric, RangeQuery, StatsRangeKey, StatsRangeResponse, TimeSplit,
            resolve_stats_range,
        },
    },
    error::{AppError, Result},
    repositories::{listening_events, response_cache, settings},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(history))
        .route("/stats/range", get(stats_range))
        .route("/stats/overview", get(overview))
        .route("/stats/dashboard", get(dashboard))
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
    let user = current_user(&headers, &state).await?;
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let rows = listening_events::history(
        &state.db,
        user.id,
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
    let user_settings = settings::get(&state.db, user.id).await?;
    let timezone_name = user_settings
        .timezone
        .clone()
        .unwrap_or_else(|| state.config.timezone.name().to_owned());
    let hour_format = user_settings.hour_format.clone();
    let timezone = timezone_name
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))?;
    let range = resolve_stats_range(timezone, query)?;
    let current_local_year = Utc::now().with_timezone(&timezone).year();
    let cache_key = overview_cache_key(&timezone_name, &hour_format, &range, current_local_year);

    if let Some(cached) = response_cache::get(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        user.id,
        &cache_key,
    )
    .await?
    {
        return Ok(Json(cached));
    }

    let start = range.start;
    let end = range.end;
    let previous_start = range.previous_start;
    let previous_end = range.previous_end;

    let previous_summary = async {
        match (previous_start, previous_end) {
            (Some(start), Some(end)) => Ok(Some(
                listening_events::summary(&state.db, user.id, Some(start), Some(end)).await?,
            )),
            _ => Ok(None),
        }
    };

    let best_artist = async {
        let mut artists =
            listening_events::top_artists(&state.db, user.id, Metric::Count, start, end, 1, 0)
                .await?;
        let best_artist = artists.pop();
        let best_artist_stats = match &best_artist {
            Some(artist) => Some(
                listening_events::entity_stats(
                    &state.db,
                    user.id,
                    listening_events::EntityFilter::Artist(&artist.id),
                    start,
                    end,
                )
                .await?,
            ),
            None => None,
        };
        Ok::<_, AppError>((best_artist, best_artist_stats))
    };

    let best_song = async {
        let mut tracks =
            listening_events::top_tracks(&state.db, user.id, Metric::Count, start, end, 1, 0)
                .await?;
        Ok::<_, AppError>(tracks.pop())
    };

    let (
        summary,
        previous_summary,
        (best_artist, best_artist_stats),
        best_song,
        hourly_distribution,
        history,
        available_years,
    ) = tokio::try_join!(
        listening_events::summary(&state.db, user.id, start, end),
        previous_summary,
        best_artist,
        best_song,
        listening_events::hour_repartition(&state.db, user.id, &timezone_name, start, end),
        listening_events::history(&state.db, user.id, start, end, 25, 0),
        available_years(&state, user.id, timezone, &timezone_name),
    )?;

    let overview = OverviewStatsResponse {
        range,
        available_years,
        summary,
        previous_summary,
        best_artist,
        best_artist_stats,
        best_song,
        hourly_distribution,
        history,
        hour_format,
        timezone: timezone_name,
    };

    response_cache::set(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        user.id,
        &cache_key,
        &overview,
        Some(Duration::days(370)),
    )
    .await?;

    Ok(Json(overview))
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
    let user = current_user(&headers, &state).await?;
    Ok(Json(dashboard_for_user(&state, user.id, query).await?))
}

pub(crate) async fn dashboard_for_user(
    state: &AppState,
    user_id: uuid::Uuid,
    query: IntervalQuery,
) -> Result<StatsDashboardResponse> {
    query.validate()?;
    let user_settings = settings::get(&state.db, user_id).await?;
    let timezone_name = user_settings
        .timezone
        .clone()
        .unwrap_or_else(|| state.config.timezone.name().to_owned());
    let hour_format = user_settings.hour_format.clone();
    let timezone = parse_timezone(&timezone_name)?;
    let (start, end) = query.resolved_bounds(timezone)?;
    let current_local_year = Utc::now().with_timezone(&timezone).year();
    let cache_key = stats_dashboard_cache_key(
        &timezone_name,
        &hour_format,
        &query,
        &start,
        &end,
        current_local_year,
    );

    if let Some(cached) = response_cache::get(
        &state.db,
        response_cache::STATS_DASHBOARD_NAMESPACE,
        user_id,
        &cache_key,
    )
    .await?
    {
        return Ok(cached);
    }

    let (
        available_years,
        summary,
        top_artists,
        artist_distribution,
        hours,
        hourly_artists,
        timeline,
        diversity,
        release_years,
        feature_average,
        feature_timeline,
    ) = tokio::try_join!(
        available_years(state, user_id, timezone, &timezone_name),
        listening_events::summary(&state.db, user_id, start, end),
        listening_events::top_artists(&state.db, user_id, Metric::Count, start, end, 8, 0),
        listening_events::top_artists_by_bucket_with_other(
            &state.db,
            user_id,
            &timezone_name,
            query.split,
            Metric::Count,
            start,
            end,
            8,
        ),
        listening_events::hour_repartition(&state.db, user_id, &timezone_name, start, end),
        listening_events::top_artists_by_hour(
            &state.db,
            user_id,
            &timezone_name,
            Metric::Count,
            start,
            end,
            1,
        ),
        listening_events::timeline(&state.db, user_id, &timezone_name, query.split, start, end),
        listening_events::diversity_timeline(
            &state.db,
            user_id,
            &timezone_name,
            query.split,
            start,
            end,
        ),
        listening_events::album_release_years(&state.db, user_id, start, end),
        listening_events::feature_average(&state.db, user_id, start, end),
        listening_events::feature_timeline(
            &state.db,
            user_id,
            &timezone_name,
            query.split,
            start,
            end
        ),
    )?;

    let dashboard = StatsDashboardResponse {
        available_years,
        summary,
        top_artists,
        artist_distribution,
        hours,
        hourly_artists,
        timeline,
        diversity,
        release_years,
        feature_average,
        feature_timeline,
        hour_format,
        timezone: timezone_name,
    };

    if matches!(
        query.range,
        Some(StatsRangeKey::All | StatsRangeKey::SelectedYear)
    ) {
        response_cache::set(
            &state.db,
            response_cache::STATS_DASHBOARD_NAMESPACE,
            user_id,
            &cache_key,
            &dashboard,
            Some(Duration::days(370)),
        )
        .await?;
    }

    Ok(dashboard)
}

fn stats_dashboard_cache_key(
    timezone: &str,
    hour_format: &str,
    query: &IntervalQuery,
    start: &Option<DateTime<Utc>>,
    end: &Option<DateTime<Utc>>,
    current_local_year: i32,
) -> String {
    format!(
        "v1:{timezone}:{hour_format}:{current_local_year}:{:?}:{}:{:?}:{}:{}",
        query.range,
        query
            .year
            .map(|year| year.to_string())
            .unwrap_or_else(|| "-".to_owned()),
        query.split,
        cache_time_part(start),
        cache_time_part(end),
    )
}

fn overview_cache_key(
    timezone: &str,
    hour_format: &str,
    range: &StatsRangeResponse,
    current_local_year: i32,
) -> String {
    format!(
        "v2:{timezone}:{hour_format}:{current_local_year}:{:?}:{}:{}:{}:{}",
        range.range,
        cache_time_part(&range.start),
        cache_time_part(&range.end),
        cache_time_part(&range.previous_start),
        cache_time_part(&range.previous_end),
    )
}

fn cache_time_part(value: &Option<DateTime<Utc>>) -> String {
    value
        .as_ref()
        .map(|date| date.to_rfc3339())
        .unwrap_or_else(|| "-".to_owned())
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
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let stats = listening_events::summary(&state.db, user.id, start, end).await?;
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
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows =
        listening_events::timeline(&state.db, user.id, &timezone, query.split, start, end).await?;
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
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::diversity_timeline(
            &state.db,
            user.id,
            &timezone,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = listening_events::top_tracks(
        &state.db,
        user.id,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = listening_events::top_artists(
        &state.db,
        user.id,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = listening_events::top_albums(
        &state.db,
        user.id,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_tracks_by_bucket(
            &state.db,
            user.id,
            &timezone,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = if query.group_other.unwrap_or(false) {
        listening_events::top_artists_by_bucket_with_other(
            &state.db,
            user.id,
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
            user.id,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows = listening_events::top_artists_by_hour(
        &state.db,
        user.id,
        &timezone,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    Ok(Json(
        listening_events::top_albums_by_bucket(
            &state.db,
            user.id,
            &timezone,
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows =
        listening_events::hour_repartition(&state.db, user.id, &timezone, start, end).await?;
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
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let stats = listening_events::feature_ratio(&state.db, user.id, start, end).await?;
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
    let user = current_user(&headers, &state).await?;
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let stats = listening_events::feature_average(&state.db, user.id, start, end).await?;
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
    let user = current_user(&headers, &state).await?;
    let timezone = user_timezone(&state, user.id).await?;
    let (start, end) = query.resolved_bounds(parse_timezone(&timezone)?)?;
    let rows =
        listening_events::feature_timeline(&state.db, user.id, &timezone, query.split, start, end)
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
    let user = current_user(&headers, &state).await?;
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let stats = listening_events::album_release_years(&state.db, user.id, start, end).await?;
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
    let (start, end) = interval_bounds(&state, user.id, &query).await?;
    let rows =
        listening_events::longest_sessions(&state.db, user.id, start, end, query.limit_or(10))
            .await?;
    Ok(Json(rows))
}

async fn interval_bounds(
    state: &AppState,
    user_id: uuid::Uuid,
    query: &IntervalQuery,
) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    if query.range.is_none() {
        return Ok((query.start, query.end));
    }
    let timezone = user_timezone(state, user_id).await?;
    query.resolved_bounds(parse_timezone(&timezone)?)
}

fn parse_timezone(value: &str) -> Result<Tz> {
    value
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))
}

async fn user_timezone(state: &AppState, user_id: uuid::Uuid) -> Result<String> {
    let user_settings = settings::get(&state.db, user_id).await?;
    Ok(user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned()))
}
