use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TimeSplit {
    All,
    Year,
    Month,
    Week,
    Day,
    Hour,
}

impl Default for TimeSplit {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Count,
    Duration,
}

impl Default for Metric {
    fn default() -> Self {
        Self::Count
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
pub struct StatsInterval {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EffectiveUserContext {
    pub user_id: Uuid,
    pub timezone: String,
    pub public: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StatsRangeKey {
    Today,
    Week,
    Month,
    Year,
    SelectedYear,
    All,
}

impl Default for StatsRangeKey {
    fn default() -> Self {
        Self::Today
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct RangeQuery {
    #[serde(default)]
    pub range: StatsRangeKey,
    pub year: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct StatsRangeResponse {
    pub range: StatsRangeKey,
    pub label: String,
    pub comparison_label: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub previous_start: Option<DateTime<Utc>>,
    pub previous_end: Option<DateTime<Utc>>,
}

pub fn resolve_stats_range(timezone: Tz, query: RangeQuery) -> Result<StatsRangeResponse> {
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&timezone);
    let today = now_local.date_naive();
    let current_year = now_local.year();

    let response = match query.range {
        StatsRangeKey::All => StatsRangeResponse {
            range: StatsRangeKey::All,
            label: "All time".to_owned(),
            comparison_label: None,
            start: None,
            end: None,
            previous_start: None,
            previous_end: None,
        },
        StatsRangeKey::Today => {
            let start_date = today;
            let previous_start = today - Duration::days(1);
            let previous_end = now_local.naive_local() - Duration::days(1);
            StatsRangeResponse {
                range: StatsRangeKey::Today,
                label: "Today".to_owned(),
                comparison_label: Some("yesterday".to_owned()),
                start: Some(local_midnight_utc(timezone, start_date)),
                end: Some(now_utc),
                previous_start: Some(local_midnight_utc(timezone, previous_start)),
                previous_end: Some(local_datetime_utc(timezone, previous_end)),
            }
        }
        StatsRangeKey::Week => {
            let days_from_monday = today.weekday().num_days_from_monday() as i64;
            let start_date = today - Duration::days(days_from_monday);
            let previous_start = start_date - Duration::days(7);
            let previous_end = now_local.naive_local() - Duration::days(7);
            StatsRangeResponse {
                range: StatsRangeKey::Week,
                label: "This week".to_owned(),
                comparison_label: Some("last week".to_owned()),
                start: Some(local_midnight_utc(timezone, start_date)),
                end: Some(now_utc),
                previous_start: Some(local_midnight_utc(timezone, previous_start)),
                previous_end: Some(local_datetime_utc(timezone, previous_end)),
            }
        }
        StatsRangeKey::Month => {
            let start_date = local_date(current_year, now_local.month(), 1)?;
            let previous_start = add_months(start_date, -1)?;
            let previous_end = add_months_to_datetime(now_local.naive_local(), -1)?;
            StatsRangeResponse {
                range: StatsRangeKey::Month,
                label: "This month".to_owned(),
                comparison_label: Some("last month".to_owned()),
                start: Some(local_midnight_utc(timezone, start_date)),
                end: Some(now_utc),
                previous_start: Some(local_midnight_utc(timezone, previous_start)),
                previous_end: Some(local_datetime_utc(timezone, previous_end)),
            }
        }
        StatsRangeKey::Year => {
            let start_date = local_date(current_year, 1, 1)?;
            let previous_start = local_date(current_year - 1, 1, 1)?;
            let previous_end = add_months_to_datetime(now_local.naive_local(), -12)?;
            StatsRangeResponse {
                range: StatsRangeKey::Year,
                label: "This year".to_owned(),
                comparison_label: Some("last year".to_owned()),
                start: Some(local_midnight_utc(timezone, start_date)),
                end: Some(now_utc),
                previous_start: Some(local_midnight_utc(timezone, previous_start)),
                previous_end: Some(local_datetime_utc(timezone, previous_end)),
            }
        }
        StatsRangeKey::SelectedYear => {
            let year = query
                .year
                .ok_or_else(|| AppError::validation("year is required for selected-year"))?;
            if !(1900..=3000).contains(&year) {
                return Err(AppError::validation("year must be between 1900 and 3000"));
            }
            let start_date = local_date(year, 1, 1)?;
            let end_date = local_date(year + 1, 1, 1)?;
            let previous_start = local_date(year - 1, 1, 1)?;
            StatsRangeResponse {
                range: StatsRangeKey::SelectedYear,
                label: year.to_string(),
                comparison_label: Some("last year".to_owned()),
                start: Some(local_midnight_utc(timezone, start_date)),
                end: Some(local_midnight_utc(timezone, end_date)),
                previous_start: Some(local_midnight_utc(timezone, previous_start)),
                previous_end: Some(local_midnight_utc(timezone, start_date)),
            }
        }
    };

    Ok(response)
}

fn local_date(year: i32, month: u32, day: u32) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| AppError::validation("invalid local date"))
}

fn add_months(date: NaiveDate, months: i32) -> Result<NaiveDate> {
    let absolute = date.year() * 12 + date.month0() as i32 + months;
    let year = absolute.div_euclid(12);
    let month0 = absolute.rem_euclid(12) as u32;
    local_date(year, month0 + 1, 1)
}

fn add_months_to_datetime(value: NaiveDateTime, months: i32) -> Result<NaiveDateTime> {
    let date = value.date();
    let absolute = date.year() * 12 + date.month0() as i32 + months;
    let year = absolute.div_euclid(12);
    let month0 = absolute.rem_euclid(12) as u32;
    let month = month0 + 1;
    let day = date.day().min(days_in_month(year, month)?);
    Ok(NaiveDateTime::new(
        local_date(year, month, day)?,
        value.time(),
    ))
}

fn days_in_month(year: i32, month: u32) -> Result<u32> {
    let next_month = if month == 12 {
        local_date(year + 1, 1, 1)?
    } else {
        local_date(year, month + 1, 1)?
    };
    Ok((next_month - Duration::days(1)).day())
}

fn local_midnight_utc(timezone: Tz, date: NaiveDate) -> chrono::DateTime<Utc> {
    let naive = date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid naive time");
    local_datetime_utc(timezone, naive)
}

fn local_datetime_utc(timezone: Tz, naive: NaiveDateTime) -> chrono::DateTime<Utc> {
    for minutes in [0, 30, 60, 90, 120, 180] {
        let candidate = naive + Duration::minutes(minutes);
        match timezone.from_local_datetime(&candidate) {
            LocalResult::Single(value) => return value.with_timezone(&Utc),
            LocalResult::Ambiguous(earliest, _) => return earliest.with_timezone(&Utc),
            LocalResult::None => {}
        }
    }
    timezone.from_utc_datetime(&naive).with_timezone(&Utc)
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct IntervalQuery {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    #[serde(default)]
    pub split: TimeSplit,
    #[serde(default)]
    pub metric: Metric,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl IntervalQuery {
    pub fn validate(&self) -> crate::error::Result<()> {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            if start >= end {
                return Err(crate::error::AppError::validation(
                    "start must be before end",
                ));
            }
        }
        if let Some(limit) = self.limit {
            if !(1..=100).contains(&limit) {
                return Err(crate::error::AppError::validation(
                    "limit must be between 1 and 100",
                ));
            }
        }
        if let Some(offset) = self.offset {
            if offset < 0 {
                return Err(crate::error::AppError::validation(
                    "offset must be positive",
                ));
            }
        }
        Ok(())
    }

    pub fn limit_or(&self, default: i64) -> i64 {
        self.limit.unwrap_or(default).clamp(1, 100)
    }

    pub fn offset_or_zero(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
