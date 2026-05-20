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
    let timezone_provided = patch.timezone.is_some();
    let timezone = patch.timezone.clone().flatten();

    let settings = sqlx::query_as::<_, UserSettings>(
        r#"
        UPDATE user_settings
        SET history_line = COALESCE($2, history_line),
            preferred_stats_period = COALESCE($3, preferred_stats_period),
            nb_elements = COALESCE($4, nb_elements),
            metric_used = COALESCE($5, metric_used),
            dark_mode = COALESCE($6, dark_mode),
            timezone = CASE WHEN $7 THEN $8 ELSE timezone END,
            date_format = COALESCE($9, date_format),
            hour_format = COALESCE($10, hour_format),
            updated_at = now()
        WHERE user_id = $1
        RETURNING user_id, history_line, preferred_stats_period, nb_elements, metric_used,
                  dark_mode, timezone, date_format, hour_format, updated_at
        "#,
    )
    .bind(user_id)
    .bind(patch.history_line)
    .bind(patch.preferred_stats_period.as_deref())
    .bind(patch.nb_elements)
    .bind(patch.metric_used.as_deref())
    .bind(patch.dark_mode.as_deref())
    .bind(timezone_provided)
    .bind(timezone)
    .bind(patch.date_format.as_deref())
    .bind(patch.hour_format.as_deref())
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
