use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Follow,
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum HourFormat {
    #[serde(rename = "12")]
    Twelve,
    #[serde(rename = "24")]
    TwentyFour,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub history_line: bool,
    pub preferred_stats_period: String,
    pub nb_elements: i32,
    pub metric_used: String,
    #[schema(value_type = ThemePreference)]
    pub dark_mode: String,
    pub timezone: Option<String>,
    pub date_format: String,
    #[schema(value_type = HourFormat)]
    pub hour_format: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GlobalPreferences {
    pub allow_registrations: bool,
    pub allow_affinity: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StatsDisplayPreferences {
    #[schema(value_type = HourFormat)]
    pub hour_format: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SettingsPatch {
    pub history_line: Option<bool>,
    pub preferred_stats_period: Option<String>,
    pub nb_elements: Option<i32>,
    pub metric_used: Option<String>,
    #[schema(value_type = Option<ThemePreference>)]
    pub dark_mode: Option<String>,
    pub timezone: Option<Option<String>>,
    pub date_format: Option<String>,
    #[schema(value_type = Option<HourFormat>)]
    pub hour_format: Option<String>,
}

impl SettingsPatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        if let Some(value) = &self.preferred_stats_period
            && !matches!(value.as_str(), "day" | "week" | "month" | "year")
        {
            return Err(crate::error::AppError::validation(
                "preferred_stats_period must be day, week, month, or year",
            ));
        }
        if let Some(value) = self.nb_elements
            && !(5..=50).contains(&value)
        {
            return Err(crate::error::AppError::validation(
                "nb_elements must be between 5 and 50",
            ));
        }
        if let Some(value) = &self.metric_used
            && !matches!(value.as_str(), "number" | "duration")
        {
            return Err(crate::error::AppError::validation(
                "metric_used must be number or duration",
            ));
        }
        if let Some(value) = &self.dark_mode
            && !matches!(value.as_str(), "follow" | "dark" | "light")
        {
            return Err(crate::error::AppError::validation(
                "dark_mode must be follow, dark, or light",
            ));
        }
        if let Some(Some(timezone)) = &self.timezone {
            timezone.parse::<chrono_tz::Tz>().map_err(|_| {
                crate::error::AppError::validation("timezone must be an IANA timezone name")
            })?;
        }
        if let Some(value) = &self.hour_format
            && !matches!(value.as_str(), "12" | "24")
        {
            return Err(crate::error::AppError::validation(
                "hour_format must be 12 or 24",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GlobalPreferencesPatch {
    pub allow_registrations: Option<bool>,
    pub allow_affinity: Option<bool>,
}

crate::impl_from_pg_row!(UserSettings {
    user_id,
    history_line,
    preferred_stats_period,
    nb_elements,
    metric_used,
    dark_mode,
    timezone,
    date_format,
    hour_format,
    updated_at,
});

crate::impl_from_pg_row!(GlobalPreferences {
    allow_registrations,
    allow_affinity,
    updated_at,
});
