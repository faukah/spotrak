use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub history_line: bool,
    pub preferred_stats_period: String,
    pub nb_elements: i32,
    pub metric_used: String,
    pub dark_mode: String,
    pub timezone: Option<String>,
    pub date_format: String,
    pub hour_format: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlobalPreferences {
    pub allow_registrations: bool,
    pub allow_affinity: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SettingsPatch {
    pub history_line: Option<bool>,
    pub preferred_stats_period: Option<String>,
    pub nb_elements: Option<i32>,
    pub metric_used: Option<String>,
    pub dark_mode: Option<String>,
    pub timezone: Option<Option<String>>,
    pub date_format: Option<String>,
    pub hour_format: Option<String>,
}

impl SettingsPatch {
    pub fn validate(&self) -> crate::error::Result<()> {
        if let Some(value) = &self.preferred_stats_period {
            if !matches!(value.as_str(), "day" | "week" | "month" | "year") {
                return Err(crate::error::AppError::validation(
                    "preferred_stats_period must be day, week, month, or year",
                ));
            }
        }
        if let Some(value) = self.nb_elements {
            if !(5..=50).contains(&value) {
                return Err(crate::error::AppError::validation(
                    "nb_elements must be between 5 and 50",
                ));
            }
        }
        if let Some(value) = &self.metric_used {
            if !matches!(value.as_str(), "number" | "duration") {
                return Err(crate::error::AppError::validation(
                    "metric_used must be number or duration",
                ));
            }
        }
        if let Some(value) = &self.dark_mode {
            if !matches!(value.as_str(), "follow" | "dark" | "light") {
                return Err(crate::error::AppError::validation(
                    "dark_mode must be follow, dark, or light",
                ));
            }
        }
        if let Some(Some(timezone)) = &self.timezone {
            timezone.parse::<chrono_tz::Tz>().map_err(|_| {
                crate::error::AppError::validation("timezone must be an IANA timezone name")
            })?;
        }
        if let Some(value) = &self.hour_format {
            if !matches!(value.as_str(), "12" | "24") {
                return Err(crate::error::AppError::validation(
                    "hour_format must be 12 or 24",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GlobalPreferencesPatch {
    pub allow_registrations: Option<bool>,
    pub allow_affinity: Option<bool>,
}
