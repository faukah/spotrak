use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ImportJob {
    pub id: Uuid,
    pub user_id: Uuid,
    pub import_type: String,
    pub status: String,
    pub total: i32,
    pub current: i32,
    pub metadata: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ImportFile {
    pub id: Uuid,
    pub job_id: Uuid,
    pub path: String,
    pub original_name: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}
