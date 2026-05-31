use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlbumRef {
    pub id: String,
    pub name: String,
    pub images: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrackDetail {
    pub id: String,
    pub name: String,
    pub duration_ms: i32,
    pub explicit: bool,
    pub href: Option<String>,
    pub uri: Option<String>,
    pub popularity: Option<i32>,
    pub disc_number: Option<i32>,
    pub track_number: Option<i32>,
    pub images: serde_json::Value,
    pub album: AlbumRef,
    pub artists: Vec<EntityRef>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    pub href: Option<String>,
    pub uri: Option<String>,
    pub popularity: Option<i32>,
    pub images: serde_json::Value,
    pub genres: serde_json::Value,
    pub blacklisted: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlbumDetail {
    pub id: String,
    pub name: String,
    pub album_type: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub total_tracks: Option<i32>,
    pub href: Option<String>,
    pub uri: Option<String>,
    pub images: serde_json::Value,
    pub artists: Vec<EntityRef>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchResults {
    pub tracks: Vec<EntityRef>,
    pub artists: Vec<EntityRef>,
    pub albums: Vec<EntityRef>,
}

crate::impl_from_pg_row!(EntityRef { id, name });
