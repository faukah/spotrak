use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::settings::{GlobalPreferences, GlobalPreferencesPatch, SettingsPatch, UserSettings},
    error::{AppError, Result},
};

pub async fn ensure_default(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<UserSettings> {
    let settings = sqlx::query_as::<_, UserSettings>(
        r#"
        INSERT INTO user_settings (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING user_id, history_line, preferred_stats_period, nb_elements, metric_used,
                  dark_mode, timezone, date_format, hour_format, updated_at
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(settings)
}

pub async fn get(pool: &PgPool, user_id: Uuid) -> Result<UserSettings> {
    let settings = sqlx::query_as::<_, UserSettings>(
        r#"
        SELECT user_id, history_line, preferred_stats_period, nb_elements, metric_used,
               dark_mode, timezone, date_format, hour_format, updated_at
        FROM user_settings
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(settings)
}

pub async fn update(pool: &PgPool, user_id: Uuid, patch: &SettingsPatch) -> Result<UserSettings> {
    patch.validate()?;
    let current = get(pool, user_id).await?;
    let timezone = patch.timezone.clone().unwrap_or(current.timezone);

    let settings = sqlx::query_as::<_, UserSettings>(
        r#"
        UPDATE user_settings
        SET history_line = $2,
            preferred_stats_period = $3,
            nb_elements = $4,
            metric_used = $5,
            dark_mode = $6,
            timezone = $7,
            date_format = $8,
            hour_format = $9,
            updated_at = now()
        WHERE user_id = $1
        RETURNING user_id, history_line, preferred_stats_period, nb_elements, metric_used,
                  dark_mode, timezone, date_format, hour_format, updated_at
        "#,
    )
    .bind(user_id)
    .bind(patch.history_line.unwrap_or(current.history_line))
    .bind(
        patch
            .preferred_stats_period
            .as_deref()
            .unwrap_or(&current.preferred_stats_period),
    )
    .bind(patch.nb_elements.unwrap_or(current.nb_elements))
    .bind(patch.metric_used.as_deref().unwrap_or(&current.metric_used))
    .bind(patch.dark_mode.as_deref().unwrap_or(&current.dark_mode))
    .bind(timezone)
    .bind(patch.date_format.as_deref().unwrap_or(&current.date_format))
    .bind(patch.hour_format.as_deref().unwrap_or(&current.hour_format))
    .fetch_one(pool)
    .await?;
    Ok(settings)
}

pub async fn global(pool: &PgPool) -> Result<GlobalPreferences> {
    let preferences = sqlx::query_as::<_, GlobalPreferences>(
        r#"
        SELECT allow_registrations, allow_affinity, updated_at
        FROM global_preferences
        WHERE id = TRUE
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(preferences)
}

pub async fn update_global(
    pool: &PgPool,
    patch: &GlobalPreferencesPatch,
) -> Result<GlobalPreferences> {
    let current = global(pool).await?;
    let preferences = sqlx::query_as::<_, GlobalPreferences>(
        r#"
        UPDATE global_preferences
        SET allow_registrations = $1,
            allow_affinity = $2,
            updated_at = now()
        WHERE id = TRUE
        RETURNING allow_registrations, allow_affinity, updated_at
        "#,
    )
    .bind(
        patch
            .allow_registrations
            .unwrap_or(current.allow_registrations),
    )
    .bind(patch.allow_affinity.unwrap_or(current.allow_affinity))
    .fetch_one(pool)
    .await?;
    Ok(preferences)
}
