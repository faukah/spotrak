use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("validation error: {message}")]
    Validation {
        message: String,
        details: Vec<Value>,
    },
    #[allow(dead_code)]
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("spotify error: {0}")]
    Spotify(String),
    #[error("spotify API error: status={status}, url={url}, body={body}")]
    SpotifyApi {
        status: reqwest::StatusCode,
        url: String,
        body: String,
    },
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error: {0}")]
    Internal(String),
    #[allow(dead_code)]
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<Value>,
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn spotify(message: impl Into<String>) -> Self {
        Self::Spotify(message.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Spotify(_) | AppError::SpotifyApi { .. } => StatusCode::BAD_GATEWAY,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::NotFound => "NOT_FOUND",
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Spotify(_) | AppError::SpotifyApi { .. } => "SPOTIFY_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
        }
    }

    fn public_message(&self) -> String {
        match self {
            AppError::Database(_) => "Database error".to_owned(),
            AppError::Internal(_) => "Internal error".to_owned(),
            AppError::Unauthorized => "Authentication required".to_owned(),
            AppError::Forbidden => "Permission denied".to_owned(),
            AppError::NotFound => "Resource not found".to_owned(),
            AppError::Validation { message, .. } => message.clone(),
            AppError::Conflict(message) => message.clone(),
            AppError::Spotify(message) => message.clone(),
            AppError::SpotifyApi { status, url, body } => {
                format!("Spotify request failed: status={status}, url={url}, body={body}")
            }
            AppError::NotImplemented(feature) => format!("{feature} is not implemented yet"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if matches!(self, AppError::Database(_) | AppError::Internal(_)) {
            tracing::error!(error = ?self, "request failed");
        }
        let details = match &self {
            AppError::Validation { details, .. } => details.clone(),
            _ => Vec::new(),
        };
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code(),
                message: self.public_message(),
                details,
            },
        };

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError::Internal(value.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        AppError::Spotify(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
