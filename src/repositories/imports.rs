use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::import::{ImportFile, ImportJob},
    error::{AppError, Result},
};

pub async fn create_job(pool: &PgPool, user_id: Uuid, import_type: &str) -> Result<ImportJob> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        INSERT INTO import_jobs (user_id, import_type, status)
        VALUES ($1, $2, 'queued')
        RETURNING id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(import_type)
    .fetch_one(pool)
    .await?;
    Ok(job)
}

pub async fn update_metadata(pool: &PgPool, job_id: Uuid, metadata: Value) -> Result<ImportJob> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        UPDATE import_jobs
        SET metadata = $2, updated_at = now()
        WHERE id = $1
        RETURNING id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        "#,
    )
    .bind(job_id)
    .bind(metadata)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(job)
}

pub async fn add_file(
    pool: &PgPool,
    job_id: Uuid,
    path: &str,
    original_name: &str,
    size_bytes: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO import_files (job_id, path, original_name, size_bytes)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(job_id)
    .bind(path)
    .bind(original_name)
    .bind(size_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn claim_next(pool: &PgPool) -> Result<Option<ImportJob>> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        WITH next_job AS (
            SELECT id
            FROM import_jobs
            WHERE status = 'queued'
               OR (status = 'progress' AND updated_at < now() - interval '5 minutes')
            ORDER BY
                CASE WHEN status = 'progress' THEN 0 ELSE 1 END,
                created_at ASC,
                id ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE import_jobs job
        SET status = 'progress', updated_at = now()
        FROM next_job
        WHERE job.id = next_job.id
        RETURNING job.id, job.user_id, job.import_type, job.status, job.total, job.current,
                  job.metadata, job.error_message, job.created_at, job.updated_at
        "#,
    )
    .fetch_optional(pool)
    .await?;
    Ok(job)
}

pub async fn files(pool: &PgPool, job_id: Uuid) -> Result<Vec<ImportFile>> {
    let files = sqlx::query_as::<_, ImportFile>(
        r#"
        SELECT id, job_id, path, original_name, size_bytes, created_at
        FROM import_files
        WHERE job_id = $1
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;
    Ok(files)
}

pub async fn delete_files(pool: &PgPool, job_id: Uuid) -> Result<u64> {
    let result = sqlx::query("DELETE FROM import_files WHERE job_id = $1")
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn list(pool: &PgPool, user_id: Uuid) -> Result<Vec<ImportJob>> {
    let jobs = sqlx::query_as::<_, ImportJob>(
        r#"
        SELECT id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        FROM import_jobs
        WHERE user_id = $1
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(jobs)
}

pub async fn get(pool: &PgPool, user_id: Uuid, job_id: Uuid) -> Result<ImportJob> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        SELECT id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        FROM import_jobs
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(job_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(job)
}

pub async fn set_status(
    pool: &PgPool,
    user_id: Uuid,
    job_id: Uuid,
    status: &str,
) -> Result<ImportJob> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        UPDATE import_jobs
        SET status = $3, updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        "#,
    )
    .bind(job_id)
    .bind(user_id)
    .bind(status)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(job)
}

pub async fn delete_import_jobs_for_user(pool: &PgPool, user_id: Uuid) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM import_jobs
        WHERE user_id = $1
          AND import_type IN ('privacy', 'full-privacy')
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn delete(pool: &PgPool, user_id: Uuid, job_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM import_jobs WHERE id = $1 AND user_id = $2")
        .bind(job_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_any(pool: &PgPool, job_id: Uuid) -> Result<ImportJob> {
    let job = sqlx::query_as::<_, ImportJob>(
        r#"
        SELECT id, user_id, import_type, status, total, current, metadata, error_message, created_at, updated_at
        FROM import_jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(job)
}

pub async fn mark_progress(pool: &PgPool, job_id: Uuid, total: i32, current: i32) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE import_jobs
        SET total = $2, current = $3, status = 'progress', updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(total)
    .bind(current)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_success(pool: &PgPool, job_id: Uuid, total: i32, current: i32) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE import_jobs
        SET total = $2, current = $3, status = 'success', error_message = NULL, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(total)
    .bind(current)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failure(pool: &PgPool, job_id: Uuid, message: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE import_jobs
        SET status = 'failure', error_message = $2, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(message)
    .execute(pool)
    .await?;
    Ok(())
}
