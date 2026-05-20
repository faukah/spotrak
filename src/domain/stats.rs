use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::domain::time::StatsRangeResponse;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OverviewStatsResponse {
    pub range: StatsRangeResponse,
    pub available_years: Vec<i32>,
    pub summary: SummaryStats,
    pub previous_summary: Option<SummaryStats>,
    pub best_artist: Option<TopArtist>,
    pub best_artist_stats: Option<EntityStats>,
    pub best_song: Option<TopTrack>,
    pub hourly_distribution: Vec<HourRepartitionPoint>,
    pub history: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct SummaryStats {
    pub total_listens: i64,
    pub total_duration_ms: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TimelinePoint {
    pub bucket: String,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TopTrack {
    pub id: String,
    pub name: String,
    pub album_id: String,
    pub album_name: String,
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TopArtist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct TopAlbum {
    pub id: String,
    pub name: String,
    pub artist_name: Option<String>,
    pub release_year: Option<i32>,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct HistoryEvent {
    pub id: uuid::Uuid,
    pub track_id: String,
    pub track_name: String,
    pub album_id: String,
    pub album_name: String,
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub duration_ms: i32,
    pub played_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct HourRepartitionPoint {
    pub hour: i32,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FeatureRatioStats {
    pub solo_count: i64,
    pub feature_count: i64,
    pub solo_duration_ms: i64,
    pub feature_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct AlbumReleaseYearPoint {
    pub release_year: Option<i32>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlbumReleaseYearsStats {
    pub average_release_year: Option<f64>,
    pub distribution: Vec<AlbumReleaseYearPoint>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DiversityTimelinePoint {
    pub bucket: String,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub average_release_year: Option<f64>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct EntityStats {
    pub total_listens: i64,
    pub total_duration_ms: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LongestSession {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_ms: i64,
    pub listens: i64,
    pub tracks: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct BucketedTopTrack {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub album_id: String,
    pub album_name: String,
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct BucketedTopArtist {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct BucketedTopAlbum {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub artist_name: Option<String>,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}
