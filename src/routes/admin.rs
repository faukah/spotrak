use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, patch},
};
use uuid::Uuid;

use crate::{
    auth::extractors::require_admin,
    domain::settings::GlobalPreferencesPatch,
    dto::{
        requests::{AdminUserPatchRequest, PaginationQuery},
        responses::{GlobalPreferencesResponse, UsersResponse},
    },
    error::{AppError, Result},
    repositories::{settings, users},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users))
        .route("/admin/users/{id}", patch(update_user).delete(delete_user))
        .route(
            "/admin/preferences",
            get(get_preferences).patch(update_preferences),
        )
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    params(PaginationQuery),
    responses((status = 200, description = "Users", body = UsersResponse))
)]
pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<UsersResponse>> {
    require_admin(&headers, &state).await?;
    let users = users::list(&state.db, query.limit_or(50), query.offset_or_zero()).await?;
    Ok(Json(UsersResponse { users }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/admin/users/{id}",
    request_body = AdminUserPatchRequest,
    responses((status = 200, description = "Updated user", body = crate::domain::user::PublicUser))
)]
pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(patch): Json<AdminUserPatchRequest>,
) -> Result<Json<crate::domain::user::PublicUser>> {
    require_admin(&headers, &state).await?;
    let admin = patch
        .admin
        .ok_or_else(|| AppError::validation("admin field is required"))?;
    let user = users::set_admin(&state.db, id, admin).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/users/{id}",
    responses((status = 204, description = "Deleted user"))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode> {
    require_admin(&headers, &state).await?;
    if users::delete(&state.db, id).await? {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/preferences",
    responses((status = 200, description = "Global preferences", body = GlobalPreferencesResponse))
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<GlobalPreferencesResponse>> {
    require_admin(&headers, &state).await?;
    let preferences = settings::global(&state.db).await?;
    Ok(Json(GlobalPreferencesResponse { preferences }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/admin/preferences",
    request_body = GlobalPreferencesPatch,
    responses((status = 200, description = "Updated global preferences", body = GlobalPreferencesResponse))
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<GlobalPreferencesPatch>,
) -> Result<Json<GlobalPreferencesResponse>> {
    require_admin(&headers, &state).await?;
    let preferences = settings::update_global(&state.db, &patch).await?;
    Ok(Json(GlobalPreferencesResponse { preferences }))
}
