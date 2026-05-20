use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub spotify_id: String,
    pub admin: bool,
    #[serde(skip_serializing)]
    #[schema(value_type = Option<String>)]
    pub access_token: Option<String>,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    #[schema(value_type = Option<String>)]
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_spotify_poll_at: Option<DateTime<Utc>>,
    pub first_listened_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub spotify_id: String,
    pub admin: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            spotify_id: user.spotify_id,
            admin: user.admin,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub spotify_id: String,
    pub admin: bool,
}
