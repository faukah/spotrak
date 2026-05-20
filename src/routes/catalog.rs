use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};

use crate::{
    auth::extractors::current_user,
    domain::{
        catalog::{AlbumDetail, ArtistDetail, SearchResults, TrackDetail},
        stats::EntityStats,
        time::IntervalQuery,
    },
    dto::requests::SearchQuery,
    error::{AppError, Result},
    repositories::{catalog, listening_events, response_cache, search, spotify_queue},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tracks/{id}", get(track))
        .route("/tracks/{id}/stats", get(track_stats))
        .route("/artists/{id}", get(artist))
        .route("/artists/{id}/stats", get(artist_stats))
        .route(
            "/artists/{id}/blacklist",
            post(blacklist_artist).delete(unblacklist_artist),
        )
        .route("/albums/{id}", get(album))
        .route("/albums/{id}/stats", get(album_stats))
        .route("/search", get(search_catalog))
}

#[utoipa::path(
    get,
    path = "/api/v1/tracks/{id}",
    responses((status = 200, description = "Track detail", body = TrackDetail))
)]
pub async fn track(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TrackDetail>> {
    let _ = current_user(&headers, &state).await?;
    Ok(Json(catalog::track(&state.db, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/artists/{id}",
    responses((status = 200, description = "Artist detail", body = ArtistDetail))
)]
pub async fn artist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ArtistDetail>> {
    let user = current_user(&headers, &state).await?;
    let detail = catalog::artist(&state.db, user.id, &id).await?;
    if let Ok(missing) = catalog::artists_missing_images(&state.db, std::slice::from_ref(&id)).await
    {
        if !missing.is_empty() {
            spotify_queue::enqueue_artist_hydration(&state.db, user.id, &missing).await?;
        }
    }
    Ok(Json(detail))
}

#[utoipa::path(
    post,
    path = "/api/v1/artists/{id}/blacklist",
    responses((status = 204, description = "Artist blacklisted"))
)]
pub async fn blacklist_artist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    catalog::blacklist_artist(&state.db, user.id, &id).await?;
    response_cache::invalidate_namespace(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        user.id,
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/v1/artists/{id}/blacklist",
    responses((status = 204, description = "Artist unblacklisted"))
)]
pub async fn unblacklist_artist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode> {
    let user = current_user(&headers, &state).await?;
    catalog::unblacklist_artist(&state.db, user.id, &id).await?;
    response_cache::invalidate_namespace(
        &state.db,
        response_cache::STATS_OVERVIEW_NAMESPACE,
        user.id,
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/albums/{id}",
    responses((status = 200, description = "Album detail", body = AlbumDetail))
)]
pub async fn album(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AlbumDetail>> {
    let _ = current_user(&headers, &state).await?;
    Ok(Json(catalog::album(&state.db, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    params(SearchQuery),
    responses((status = 200, description = "Search results", body = SearchResults))
)]
pub async fn search_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResults>> {
    let _ = current_user(&headers, &state).await?;
    if query.q.trim().is_empty() {
        return Err(AppError::validation("q must not be empty"));
    }
    Ok(Json(search::search(&state.db, query.q.trim(), 10).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/tracks/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Track stats", body = EntityStats))
)]
pub async fn track_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    catalog::track(&state.db, &id).await?;
    let user = current_user(&headers, &state).await?;
    let stats = listening_events::entity_stats(
        &state.db,
        user.id,
        listening_events::EntityFilter::Track(&id),
        query.start,
        query.end,
    )
    .await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/artists/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Artist stats", body = EntityStats))
)]
pub async fn artist_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    let user = current_user(&headers, &state).await?;
    catalog::artist(&state.db, user.id, &id).await?;
    let stats = listening_events::entity_stats(
        &state.db,
        user.id,
        listening_events::EntityFilter::Artist(&id),
        query.start,
        query.end,
    )
    .await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/albums/{id}/stats",
    params(IntervalQuery),
    responses((status = 200, description = "Album stats", body = EntityStats))
)]
pub async fn album_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<IntervalQuery>,
) -> Result<Json<EntityStats>> {
    query.validate()?;
    catalog::album(&state.db, &id).await?;
    let user = current_user(&headers, &state).await?;
    let stats = listening_events::entity_stats(
        &state.db,
        user.id,
        listening_events::EntityFilter::Album(&id),
        query.start,
        query.end,
    )
    .await?;
    Ok(Json(stats))
}
