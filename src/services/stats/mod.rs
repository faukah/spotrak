use std::collections::BTreeSet;

use axum::http::HeaderMap;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Timelike, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{
    auth::extractors::current_user,
    domain::{
        stats::{
            OverviewStatsResponse, StatsBucketAxis, StatsDashboardBootstrapResponse,
            StatsDashboardResponse,
        },
        time::{
            IntervalQuery, Metric, RangeQuery, StatsRangeKey, StatsRangeResponse, TimeSplit,
            resolve_stats_range,
        },
    },
    error::{AppError, Result},
    repositories::{listening_events, public_tokens, response_cache, settings},
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct StatsAccessContext {
    pub user_id: Uuid,
    pub timezone_name: String,
    pub timezone: Tz,
    pub hour_format: String,
}

const DASHBOARD_ARTIST_DISTRIBUTION_LIMIT: i64 = 100;
const DASHBOARD_COMEBACK_ARTISTS_LIMIT: i64 = 5;

impl StatsAccessContext {
    pub fn interval_bounds(
        &self,
        query: &IntervalQuery,
    ) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        query.resolved_bounds(self.timezone)
    }

    pub fn resolved_range(&self, query: &IntervalQuery) -> Result<Option<StatsRangeResponse>> {
        query
            .range
            .map(|range| {
                resolve_stats_range(
                    self.timezone,
                    RangeQuery {
                        range,
                        year: query.year,
                    },
                )
            })
            .transpose()
    }
}

pub async fn private_context(headers: &HeaderMap, state: &AppState) -> Result<StatsAccessContext> {
    let user = current_user(headers, state).await?;
    context_for_user(state, user.id).await
}

pub async fn public_context(state: &AppState, token: &str) -> Result<StatsAccessContext> {
    let user_id = public_tokens::user_id_for_token(&state.db, token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    context_for_user(state, user_id).await
}

pub async fn context_for_user(state: &AppState, user_id: Uuid) -> Result<StatsAccessContext> {
    let user_settings = settings::get(&state.db, user_id).await?;
    let timezone_name = user_settings
        .timezone
        .unwrap_or_else(|| state.config.timezone.name().to_owned());
    let timezone = parse_timezone(&timezone_name)?;
    Ok(StatsAccessContext {
        user_id,
        timezone_name,
        timezone,
        hour_format: user_settings.hour_format,
    })
}

pub fn parse_timezone(value: &str) -> Result<Tz> {
    value
        .parse::<Tz>()
        .map_err(|_| AppError::validation("user timezone must be an IANA timezone name"))
}

pub async fn overview_for_context(
    state: &AppState,
    context: &StatsAccessContext,
    query: RangeQuery,
) -> Result<OverviewStatsResponse> {
    let range = resolve_stats_range(context.timezone, query)?;
    let current_local_year = Utc::now().with_timezone(&context.timezone).year();
    let cache_key = overview_cache_key(
        &context.timezone_name,
        &context.hour_format,
        &range,
        current_local_year,
    );

    if let Some(cached) = response_cache::get(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        context.user_id,
        &cache_key,
    )
    .await?
    {
        return Ok(cached);
    }

    let start = range.start;
    let end = range.end;
    let previous_start = range.previous_start;
    let previous_end = range.previous_end;

    let previous_summary = async {
        match (previous_start, previous_end) {
            (Some(start), Some(end)) => Ok(Some(
                listening_events::summary(&state.db, context.user_id, Some(start), Some(end))
                    .await?,
            )),
            _ => Ok(None),
        }
    };

    let best_artist = async {
        let mut artists = listening_events::top_artists(
            &state.db,
            context.user_id,
            Metric::Count,
            start,
            end,
            1,
            0,
        )
        .await?;
        let best_artist = artists.pop();
        let best_artist_stats = match &best_artist {
            Some(artist) => Some(
                listening_events::entity_stats(
                    &state.db,
                    context.user_id,
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
        let mut tracks = listening_events::top_tracks(
            &state.db,
            context.user_id,
            Metric::Count,
            start,
            end,
            1,
            0,
        )
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
        listening_events::summary(&state.db, context.user_id, start, end),
        previous_summary,
        best_artist,
        best_song,
        listening_events::hour_repartition(
            &state.db,
            context.user_id,
            &context.timezone_name,
            start,
            end
        ),
        listening_events::history(&state.db, context.user_id, start, end, 25, 0),
        available_years(state, context),
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
        hour_format: context.hour_format.clone(),
        timezone: context.timezone_name.clone(),
    };

    response_cache::set(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        context.user_id,
        &cache_key,
        &overview,
        Some(Duration::days(370)),
    )
    .await?;

    Ok(overview)
}

pub async fn dashboard_for_context(
    state: &AppState,
    context: &StatsAccessContext,
    query: IntervalQuery,
) -> Result<StatsDashboardResponse> {
    query.validate()?;
    let (start, end) = context.interval_bounds(&query)?;
    let resolved_range = context.resolved_range(&query)?;
    let current_local_year = Utc::now().with_timezone(&context.timezone).year();
    let cache_key = stats_dashboard_cache_key(
        &context.timezone_name,
        &context.hour_format,
        &query,
        &start,
        &end,
        current_local_year,
    );

    if let Some(cached) = response_cache::get(
        &state.db,
        response_cache::STATS_DASHBOARD_NAMESPACE,
        context.user_id,
        &cache_key,
    )
    .await?
    {
        return Ok(cached);
    }

    let (
        available_years,
        summary,
        discovery,
        artist_distribution,
        hours,
        hourly_artists,
        timeline,
        diversity,
        release_years,
        feature_average,
        feature_timeline,
        behavior,
        concentration,
        comeback_artists,
    ) = tokio::try_join!(
        available_years(state, context),
        listening_events::summary(&state.db, context.user_id, start, end),
        listening_events::discovery_stats(&state.db, context.user_id, start, end),
        listening_events::top_artists_by_bucket_with_other(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            Metric::Count,
            start,
            end,
            DASHBOARD_ARTIST_DISTRIBUTION_LIMIT,
        ),
        listening_events::hour_repartition(
            &state.db,
            context.user_id,
            &context.timezone_name,
            start,
            end
        ),
        listening_events::top_artists_by_hour(
            &state.db,
            context.user_id,
            &context.timezone_name,
            Metric::Count,
            start,
            end,
            1,
        ),
        listening_events::timeline(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            start,
            end
        ),
        listening_events::diversity_timeline(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            start,
            end,
        ),
        listening_events::album_release_years(&state.db, context.user_id, start, end),
        listening_events::feature_average(&state.db, context.user_id, start, end),
        listening_events::feature_timeline(
            &state.db,
            context.user_id,
            &context.timezone_name,
            query.split,
            start,
            end
        ),
        listening_events::listening_behavior_stats(&state.db, context.user_id, start, end),
        listening_events::concentration_stats(&state.db, context.user_id, start, end),
        listening_events::comeback_artists(
            &state.db,
            context.user_id,
            start,
            end,
            DASHBOARD_COMEBACK_ARTISTS_LIMIT,
        ),
    )?;
    let (sessions, repeat_loops) = behavior;

    let raw_buckets = artist_distribution
        .iter()
        .map(|row| row.bucket.as_str())
        .chain(timeline.iter().map(|row| row.bucket.as_str()))
        .chain(diversity.iter().map(|row| row.bucket.as_str()))
        .chain(feature_timeline.iter().map(|row| row.bucket.as_str()));
    let bucket_axis = StatsBucketAxis {
        split: query.split,
        buckets: bucket_axis(context.timezone, query.split, start, end, raw_buckets),
    };

    let dashboard = StatsDashboardResponse {
        range: resolved_range,
        bucket_axis,
        available_years,
        summary,
        discovery,
        artist_distribution,
        hours,
        hourly_artists,
        timeline,
        diversity,
        release_years,
        feature_average,
        feature_timeline,
        sessions,
        concentration,
        comeback_artists,
        repeat_loops,
        hour_format: context.hour_format.clone(),
        timezone: context.timezone_name.clone(),
    };

    if matches!(
        query.range,
        Some(StatsRangeKey::All | StatsRangeKey::SelectedYear)
    ) {
        response_cache::set(
            &state.db,
            response_cache::STATS_DASHBOARD_NAMESPACE,
            context.user_id,
            &cache_key,
            &dashboard,
            Some(Duration::days(370)),
        )
        .await?;
    }

    Ok(dashboard)
}

pub async fn dashboard_bootstrap_for_context(
    state: &AppState,
    context: &StatsAccessContext,
    query: IntervalQuery,
) -> Result<StatsDashboardBootstrapResponse> {
    query.validate()?;
    let (start, end) = context.interval_bounds(&query)?;
    let resolved_range = context.resolved_range(&query)?;
    let (available_years, summary, release_years, feature_average) = tokio::try_join!(
        available_years(state, context),
        listening_events::summary(&state.db, context.user_id, start, end),
        listening_events::album_release_years(&state.db, context.user_id, start, end),
        listening_events::feature_average(&state.db, context.user_id, start, end),
    )?;

    Ok(StatsDashboardBootstrapResponse {
        range: resolved_range,
        bucket_axis: StatsBucketAxis {
            split: query.split,
            buckets: bucket_axis(
                context.timezone,
                query.split,
                start,
                end,
                std::iter::empty(),
            ),
        },
        available_years,
        summary,
        release_years,
        feature_average,
        hour_format: context.hour_format.clone(),
        timezone: context.timezone_name.clone(),
    })
}

pub async fn available_years(state: &AppState, context: &StatsAccessContext) -> Result<Vec<i32>> {
    let now_year = Utc::now().with_timezone(&context.timezone).year();
    let timeline = listening_events::timeline(
        &state.db,
        context.user_id,
        &context.timezone_name,
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

fn stats_dashboard_cache_key(
    timezone: &str,
    hour_format: &str,
    query: &IntervalQuery,
    start: &Option<DateTime<Utc>>,
    end: &Option<DateTime<Utc>>,
    current_local_year: i32,
) -> String {
    format!(
        "v5:{timezone}:{hour_format}:{current_local_year}:{:?}:{}:{:?}:{}:{}",
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
        .map(DateTime::to_rfc3339)
        .unwrap_or_else(|| "-".to_owned())
}

fn bucket_axis<'a>(
    timezone: Tz,
    split: TimeSplit,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    raw_buckets: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let raw = raw_buckets
        .filter(|bucket| !bucket.trim().is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if let (Some(start), Some(end)) = (start, end) {
        if let Some(axis) = bounded_bucket_axis(timezone, split, start, end) {
            return axis;
        }
    }
    raw.into_iter().collect()
}

fn bounded_bucket_axis(
    timezone: Tz,
    split: TimeSplit,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Option<Vec<String>> {
    if start >= end {
        return Some(Vec::new());
    }
    let grain = axis_grain(split);
    let mut cursor = truncate_local(start.with_timezone(&timezone).naive_local(), grain);
    let end_local = end.with_timezone(&timezone).naive_local() - Duration::milliseconds(1);
    let end_bucket = truncate_local(end_local, grain);
    if cursor > end_bucket {
        return Some(Vec::new());
    }

    let mut buckets = Vec::new();
    while cursor <= end_bucket {
        buckets.push(format_bucket(cursor));
        if buckets.len() > 1_200 {
            break;
        }
        cursor = add_bucket(cursor, grain)?;
    }
    Some(buckets)
}

#[derive(Debug, Clone, Copy)]
enum AxisGrain {
    Year,
    Month,
    Week,
    Day,
    Hour,
}

fn axis_grain(split: TimeSplit) -> AxisGrain {
    match split {
        TimeSplit::All | TimeSplit::Month => AxisGrain::Month,
        TimeSplit::Year => AxisGrain::Year,
        TimeSplit::Week => AxisGrain::Week,
        TimeSplit::Day => AxisGrain::Day,
        TimeSplit::Hour => AxisGrain::Hour,
    }
}

fn truncate_local(value: NaiveDateTime, grain: AxisGrain) -> NaiveDateTime {
    let date = value.date();
    let hour = if matches!(grain, AxisGrain::Hour) {
        value.hour()
    } else {
        0
    };
    let date = match grain {
        AxisGrain::Year => date.with_month(1).and_then(|date| date.with_day(1)),
        AxisGrain::Month => date.with_day(1),
        AxisGrain::Week => {
            Some(date - Duration::days(date.weekday().num_days_from_monday() as i64))
        }
        AxisGrain::Day | AxisGrain::Hour => Some(date),
    }
    .expect("existing local date can be truncated to a valid bucket");
    date.and_hms_opt(hour, 0, 0)
        .expect("bucket hour is always valid")
}

fn add_bucket(value: NaiveDateTime, grain: AxisGrain) -> Option<NaiveDateTime> {
    match grain {
        AxisGrain::Year => value.with_year(value.year() + 1),
        AxisGrain::Month => {
            let absolute = value.year() * 12 + value.month0() as i32 + 1;
            let year = absolute.div_euclid(12);
            let month0 = absolute.rem_euclid(12) as u32;
            value.with_year(year)?.with_month0(month0)
        }
        AxisGrain::Week => Some(value + Duration::weeks(1)),
        AxisGrain::Day => Some(value + Duration::days(1)),
        AxisGrain::Hour => Some(value + Duration::hours(1)),
    }
}

fn format_bucket(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn bucket_axis_includes_last_bucket_before_exclusive_end() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let axis = bounded_bucket_axis(chrono_tz::UTC, TimeSplit::Month, start, end).unwrap();

        assert_eq!(
            axis.first().map(String::as_str),
            Some("2024-01-01T00:00:00")
        );
        assert_eq!(axis.last().map(String::as_str), Some("2024-12-01T00:00:00"));
        assert_eq!(axis.len(), 12);
    }
}
