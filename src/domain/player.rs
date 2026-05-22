use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CurrentlyPlayingUnavailableReason {
    NotPlaying,
    ReconnectRequired,
    SpotifyUnavailable,
    UnsupportedItem,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CurrentlyPlayingTrack {
    pub id: String,
    pub name: String,
    pub album_id: Option<String>,
    pub album_name: String,
    pub artist_name: Option<String>,
    pub image_url: Option<String>,
    pub duration_ms: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CurrentlyPlayingResponse {
    pub fetched_at: DateTime<Utc>,
    pub is_playing: bool,
    pub progress_ms: Option<i32>,
    pub track: Option<CurrentlyPlayingTrack>,
    pub unavailable_reason: Option<CurrentlyPlayingUnavailableReason>,
}
