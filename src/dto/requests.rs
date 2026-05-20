use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfilePatchRequest {
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUserPatchRequest {
    pub admin: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl PaginationQuery {
    pub fn limit_or(&self, default: i64) -> i64 {
        self.limit.unwrap_or(default).clamp(1, 100)
    }

    pub fn offset_or_zero(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SpotifyPlayRequest {
    pub uri: Option<String>,
    pub track_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SpotifyCreatePlaylistRequest {
    pub name: String,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SpotifyAddTracksRequest {
    pub uris: Vec<String>,
}
