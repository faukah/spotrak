use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SpotifyTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SpotifyProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub href: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredSpotifyTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SpotifyImage {
    pub url: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SpotifySimpleArtist {
    pub id: Option<String>,
    pub name: String,
    pub href: Option<String>,
    pub uri: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SpotifyAlbum {
    pub id: Option<String>,
    pub name: String,
    pub album_type: Option<String>,
    pub release_date: Option<String>,
    pub release_date_precision: Option<String>,
    pub total_tracks: Option<i32>,
    pub href: Option<String>,
    pub uri: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    #[serde(default)]
    pub images: Vec<SpotifyImage>,
    #[serde(default)]
    pub artists: Vec<SpotifySimpleArtist>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SpotifyTrack {
    pub id: Option<String>,
    pub name: String,
    pub album: SpotifyAlbum,
    #[serde(default)]
    pub artists: Vec<SpotifySimpleArtist>,
    pub duration_ms: i32,
    #[serde(default)]
    pub explicit: bool,
    pub href: Option<String>,
    pub uri: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub popularity: Option<i32>,
    pub disc_number: Option<i32>,
    pub track_number: Option<i32>,
    #[serde(default)]
    pub is_local: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyRecentlyPlayedItem {
    pub track: SpotifyTrack,
    pub played_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotifyRecentlyPlayedResponse {
    #[serde(default)]
    pub items: Vec<SpotifyRecentlyPlayedItem>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotifyCurrentlyPlayingResponse {
    pub progress_ms: Option<i32>,
    #[serde(default)]
    pub is_playing: bool,
    pub currently_playing_type: Option<String>,
    pub item: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpotifyArtist {
    pub id: String,
    pub name: String,
    pub href: Option<String>,
    pub uri: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub popularity: Option<i32>,
    #[serde(default)]
    pub images: Vec<SpotifyImage>,
    #[serde(default)]
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotifySearchTracks {
    pub tracks: SpotifySearchTrackItems,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotifySearchTrackItems {
    #[serde(default)]
    pub items: Vec<SpotifyTrack>,
}
