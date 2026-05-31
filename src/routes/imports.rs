use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    auth::extractors::current_user,
    domain::import::ImportJob,
    dto::responses::{ImportJobResponse, ImportJobsResponse},
    error::{AppError, Result},
    repositories::{imports, listening_events, response_cache},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/imports/privacy", post(upload_privacy))
        .route("/imports/full-privacy", post(upload_full_privacy))
        .route("/imports", get(list_imports))
        .route(
            "/imports/history",
            axum::routing::delete(delete_imported_history),
        )
        .route("/imports/{id}", get(get_import).delete(delete_import))
        .route("/imports/{id}/retry", post(retry_import))
        .route("/imports/{id}/cancel", post(cancel_import))
}

#[utoipa::path(
    post,
    path = "/api/v1/imports/privacy",
    responses((status = 200, description = "Queued privacy imports", body = ImportJobsResponse))
)]
pub async fn upload_privacy(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Json<ImportJobsResponse>> {
    upload(state, headers, multipart, "privacy").await
}

#[utoipa::path(
    post,
    path = "/api/v1/imports/full-privacy",
    responses((status = 200, description = "Queued full privacy imports", body = ImportJobsResponse))
)]
pub async fn upload_full_privacy(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Json<ImportJobsResponse>> {
    upload(state, headers, multipart, "full-privacy").await
}

async fn upload(
    state: AppState,
    headers: HeaderMap,
    mut multipart: Multipart,
    import_type: &str,
) -> Result<Json<ImportJobsResponse>> {
    let user = current_user(&headers, &state).await?;
    tokio::fs::create_dir_all(&state.config.import_dir)
        .await
        .map_err(|err| AppError::internal(err.to_string()))?;

    let mut jobs = Vec::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::validation(err.to_string()))?
    {
        let Some(original) = field.file_name().map(str::to_owned) else {
            continue;
        };
        let safe_name = sanitize_filename(&original)?;
        let mut job = imports::create_job(&state.db, user.id, import_type).await?;
        let job_dir = state.config.import_dir.join(job.id.to_string());
        tokio::fs::create_dir_all(&job_dir)
            .await
            .map_err(|err| AppError::internal(err.to_string()))?;

        let path = job_dir.join(format!("{}-{}", Uuid::new_v4(), safe_name));
        let mut file = tokio::fs::File::create(&path)
            .await
            .map_err(|err| AppError::internal(err.to_string()))?;
        let mut size = 0_i64;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|err| AppError::validation(err.to_string()))?
        {
            size +=
                i64::try_from(chunk.len()).map_err(|err| AppError::internal(err.to_string()))?;
            if size as u64 > state.config.max_import_cache_size {
                let _ = tokio::fs::remove_file(&path).await;
                let _ = imports::delete(&state.db, user.id, job.id).await;
                let _ = tokio::fs::remove_dir_all(&job_dir).await;
                return Err(AppError::validation(
                    "uploaded file exceeds MAX_IMPORT_CACHE_SIZE",
                ));
            }
            file.write_all(&chunk)
                .await
                .map_err(|err| AppError::internal(err.to_string()))?;
        }
        file.flush()
            .await
            .map_err(|err| AppError::internal(err.to_string()))?;
        if size == 0 {
            let _ = tokio::fs::remove_file(&path).await;
            let _ = imports::delete(&state.db, user.id, job.id).await;
            let _ = tokio::fs::remove_dir_all(&job_dir).await;
            continue;
        }

        imports::add_file(&state.db, job.id, &path.to_string_lossy(), &original, size).await?;
        job = imports::update_metadata(
            &state.db,
            job.id,
            json!({
                "name": original,
                "filenames": [original],
            }),
        )
        .await?;
        jobs.push(job.into());
    }

    if jobs.is_empty() {
        return Err(AppError::validation(
            "upload must include at least one file",
        ));
    }

    let queued_jobs = jobs.len();
    let worker_state = state.clone();
    tokio::spawn(async move {
        let mut processed_total = 0_usize;
        for _ in 0..queued_jobs {
            match crate::services::imports::process_queued_once(&worker_state).await {
                Ok(processed) if processed > 0 => {
                    processed_total += processed;
                    worker_state
                        .metrics
                        .inc_import_jobs_processed(processed as u64);
                }
                Ok(_) => break,
                Err(error) => {
                    tracing::warn!(?error, "immediate import worker failed");
                    break;
                }
            }
        }
        if processed_total > 0 {
            tracing::debug!(
                processed = processed_total,
                "processed import jobs after upload"
            );
        }
    });

    Ok(Json(ImportJobsResponse { imports: jobs }))
}

#[utoipa::path(
    get,
    path = "/api/v1/imports",
    responses((status = 200, description = "Import jobs", body = ImportJobsResponse))
)]
pub async fn list_imports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ImportJobsResponse>> {
    let user = current_user(&headers, &state).await?;
    let imports = imports::list(&state.db, user.id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(ImportJobsResponse { imports }))
}

#[utoipa::path(
    get,
    path = "/api/v1/imports/{id}",
    responses((status = 200, description = "Import job", body = ImportJobResponse))
)]
pub async fn get_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>> {
    let user = current_user(&headers, &state).await?;
    Ok(Json(imports::get(&state.db, user.id, id).await?.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/imports/{id}/retry",
    responses((status = 200, description = "Retried import", body = ImportJobResponse))
)]
pub async fn retry_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>> {
    let user = current_user(&headers, &state).await?;
    let job = imports::get(&state.db, user.id, id).await?;
    if job.status != "failure" && job.status != "cancelled" {
        return Err(AppError::validation(
            "only failed or cancelled imports can be retried",
        ));
    }
    Ok(Json(
        imports::set_status(&state.db, user.id, id, "queued")
            .await?
            .into(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/imports/{id}/cancel",
    responses((status = 200, description = "Cancelled import", body = ImportJobResponse))
)]
pub async fn cancel_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>> {
    let user = current_user(&headers, &state).await?;
    let job = imports::get(&state.db, user.id, id).await?;
    if job.status != "queued" && job.status != "progress" {
        return Err(AppError::validation(
            "only queued or running imports can be cancelled",
        ));
    }
    Ok(Json(
        imports::set_status(&state.db, user.id, id, "cancelled")
            .await?
            .into(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/imports/history",
    responses((status = 204, description = "Deleted imported listening history"))
)]
pub async fn delete_imported_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    let import_jobs = imports::list(&state.db, user.id).await?;
    let mut import_files = Vec::new();
    for job in &import_jobs {
        if job.import_type == "privacy" || job.import_type == "full-privacy" {
            import_files.extend(imports::files(&state.db, job.id).await?);
        }
    }

    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;
    let deleted = listening_events::delete_imported_history(&tx, user.id).await?;
    tx.commit().await?;
    imports::delete_import_jobs_for_user(&state.db, user.id).await?;
    for file in import_files {
        if let Err(err) = tokio::fs::remove_file(&file.path).await {
            tracing::debug!(?err, path = %file.path, "failed to delete raw import file");
        }
    }
    for job in import_jobs {
        if job.import_type == "privacy" || job.import_type == "full-privacy" {
            let job_dir = state.config.import_dir.join(job.id.to_string());
            if let Err(err) = tokio::fs::remove_dir_all(&job_dir).await {
                tracing::debug!(?err, path = %job_dir.display(), "failed to delete import directory");
            }
        }
    }
    if deleted > 0 {
        response_cache::invalidate_stats(&state.db, user.id).await?;
    }
    tracing::info!(user_id = %user.id, deleted, "deleted imported listening history");
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/v1/imports/{id}",
    responses((status = 204, description = "Deleted import"))
)]
pub async fn delete_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    let files = imports::files(&state.db, id).await?;
    if imports::delete(&state.db, user.id, id).await? {
        response_cache::invalidate_stats(&state.db, user.id).await?;
        for file in files {
            if let Err(err) = tokio::fs::remove_file(&file.path).await {
                tracing::debug!(?err, path = %file.path, "failed to delete import file");
            }
        }
        let job_dir = state.config.import_dir.join(id.to_string());
        if let Err(err) = tokio::fs::remove_dir_all(&job_dir).await {
            tracing::debug!(?err, path = %job_dir.display(), "failed to delete import directory");
        }
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

fn import_name(filenames: &[String]) -> String {
    match filenames {
        [] => "Import".to_owned(),
        [single] => single.clone(),
        many => {
            let preview = many.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            if many.len() > 3 {
                format!("{preview}, +{} more", many.len() - 3)
            } else {
                preview
            }
        }
    }
}

fn sanitize_filename(name: &str) -> Result<String> {
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(128)
        .collect::<String>();
    if sanitized.is_empty() || matches!(sanitized.as_str(), "." | "..") {
        return Err(AppError::validation("invalid import filename"));
    }
    Ok(sanitized)
}

impl From<ImportJob> for ImportJobResponse {
    fn from(job: ImportJob) -> Self {
        let filenames = job
            .metadata
            .get("filenames")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty())
            .unwrap_or_default();
        let name = job
            .metadata
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if filenames.is_empty() {
                    format!("{} import", job.import_type)
                } else {
                    import_name(&filenames)
                }
            });
        Self {
            id: job.id,
            name,
            filenames,
            import_type: job.import_type,
            status: job.status,
            total: job.total,
            current: job.current,
            error_message: job.error_message,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}
