use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::domain::{
    settings::HourFormat,
    time::{StatsRangeResponse, TimeSplit},
};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
    #[schema(value_type = HourFormat)]
    pub hour_format: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct StatsDashboardResponse {
    pub range: Option<StatsRangeResponse>,
    pub bucket_axis: StatsBucketAxis,
    pub available_years: Vec<i32>,
    pub summary: SummaryStats,
    pub discovery: DiscoveryStats,
    pub artist_distribution: Vec<BucketedTopArtist>,
    pub hours: Vec<HourRepartitionPoint>,
    pub hourly_artists: Vec<HourlyTopArtist>,
    pub timeline: Vec<TimelinePoint>,
    pub diversity: Vec<DiversityTimelinePoint>,
    pub release_years: AlbumReleaseYearsStats,
    pub feature_average: FeatureAverageStats,
    pub feature_timeline: Vec<FeatureTimelinePoint>,
    pub sessions: ListeningSessionStats,
    pub concentration: ListeningConcentrationStats,
    pub comeback_artists: Vec<ComebackArtist>,
    pub repeat_loops: RepeatLoopStats,
    #[schema(value_type = HourFormat)]
    pub hour_format: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct StatsDashboardBootstrapResponse {
    pub range: Option<StatsRangeResponse>,
    pub bucket_axis: StatsBucketAxis,
    pub available_years: Vec<i32>,
    pub summary: SummaryStats,
    pub release_years: AlbumReleaseYearsStats,
    pub feature_average: FeatureAverageStats,
    #[schema(value_type = HourFormat)]
    pub hour_format: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct StatsBucketAxis {
    pub split: TimeSplit,
    pub buckets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct SummaryStats {
    pub total_listens: i64,
    pub total_duration_ms: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct DiscoveryStats {
    pub total_listens: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub new_tracks: i64,
    pub new_artists: i64,
    pub repeat_listens: i64,
    pub discovery_share: f64,
    pub repeat_share: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct TimelinePoint {
    pub bucket: String,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct TopArtist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct TopAlbum {
    pub id: String,
    pub name: String,
    pub artist_name: Option<String>,
    pub release_year: Option<i32>,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct HourRepartitionPoint {
    pub hour: i32,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct FeatureRatioStats {
    pub solo_count: i64,
    pub feature_count: i64,
    pub solo_duration_ms: i64,
    pub feature_duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct FeatureAverageStats {
    pub unique_tracks: i64,
    pub featured_tracks: i64,
    pub total_features: i64,
    pub average_features_per_song: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct FeatureTimelinePoint {
    pub bucket: String,
    pub unique_tracks: i64,
    pub featured_tracks: i64,
    pub total_features: i64,
    pub average_features_per_song: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct AlbumReleaseYearPoint {
    pub release_year: Option<i32>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AlbumReleaseYearsStats {
    pub average_release_year: Option<f64>,
    pub distribution: Vec<AlbumReleaseYearPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct DiversityTimelinePoint {
    pub bucket: String,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub average_release_year: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct EntityStats {
    pub total_listens: i64,
    pub total_duration_ms: i64,
    pub unique_tracks: i64,
    pub unique_artists: i64,
    pub unique_albums: i64,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LongestSession {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_ms: i64,
    pub listens: i64,
    pub tracks: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ListeningSessionSummary {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_ms: i64,
    pub listens: i64,
    pub unique_artists: i64,
    pub first_track_name: String,
    pub last_track_name: String,
    pub image_url: Option<String>,
    pub listens_per_hour: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ListeningSessionStats {
    pub total_sessions: i64,
    pub average_duration_ms: i64,
    pub average_listens: f64,
    pub longest: Option<ListeningSessionSummary>,
    pub most_intense: Option<ListeningSessionSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct ListeningConcentrationStats {
    pub total_listens: i64,
    pub artist_count: i64,
    pub top_artist_id: Option<String>,
    pub top_artist_name: Option<String>,
    pub top_artist_image_url: Option<String>,
    pub top_artist_listens: i64,
    pub top_artist_share: f64,
    pub top_five_share: f64,
    pub top_ten_share: f64,
    pub effective_artist_count: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct ComebackArtist {
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub gap_ms: i64,
    pub previous_played_at: DateTime<Utc>,
    pub returned_at: DateTime<Utc>,
    pub range_listens: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RepeatLoopSummary {
    pub track_id: String,
    pub track_name: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub span_ms: i64,
    pub listening_duration_ms: i64,
    pub listens: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ArtistRunSummary {
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub span_ms: i64,
    pub listening_duration_ms: i64,
    pub listens: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RepeatLoopStats {
    pub total_back_to_back_repeats: i64,
    pub top_track_loop: Option<RepeatLoopSummary>,
    pub back_to_back_track_run: Option<RepeatLoopSummary>,
    pub longest_artist_run: Option<ArtistRunSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct BucketedTopArtist {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct HourlyTopArtist {
    pub hour: i32,
    pub artist_id: String,
    pub artist_name: String,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
    pub rank: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct BucketedTopAlbum {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub artist_name: Option<String>,
    pub image_url: Option<String>,
    pub count: i64,
    pub duration_ms: i64,
}
