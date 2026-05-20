use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    settings::{GlobalPreferences, UserSettings},
    user::PublicUser,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VersionResponse {
    pub version: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub user: PublicUser,
    pub settings: UserSettings,
    pub public_sharing: PublicSharingResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicSharingResponse {
    pub enabled: bool,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UsersResponse {
    pub users: Vec<PublicUser>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GlobalPreferencesResponse {
    pub preferences: GlobalPreferences,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportJobResponse {
    pub id: Uuid,
    pub name: String,
    pub filenames: Vec<String>,
    pub import_type: String,
    pub status: String,
    pub total: i32,
    pub current: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportJobsResponse {
    pub imports: Vec<ImportJobResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicTokenResponse {
    pub token: String,
}
