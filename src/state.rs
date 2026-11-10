use std::sync::Arc;
use sqlx::postgres::{PgPool, PgPoolOptions};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::error::DatabaseError;
use serde_json::json;

use yahoo_finance_api as yahoo;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),        // 400 — malformed request (bad JSON, wrong type)
    Unauthorized,              // 401 — no/!invalid credentials
    Forbidden,                 // 403 — authenticated but not allowed
    NotFound,                  // 404 — resource does not exist
    Conflict(String),          // 409 — duplicate, state conflict
    Validation(String),  
    
    Internal(String),
    Yahoo(yahoo::YahooError),
    Database(sqlx::Error)
    
    
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        if matches!(e, sqlx::Error::RowNotFound) {
            return AppError::NotFound;
        }

        if let sqlx::Error::Database(db) = &e {
            match db.code().as_deref() {
                Some("23503") => return AppError::NotFound,                  // FK violation
                Some("23505") => return AppError::Conflict("duplicate".into()), // unique
                Some("23514") => return AppError::Validation("check constraint failed".into()),
                Some("23502") => return AppError::Validation("missing required field".into()),
                Some("22P02") => return AppError::Validation("invalid value".into()), // bad enum/uuid
                Some("22003") => return AppError::Validation("numeric out of range".into()),
                Some("40001") | Some("40P01") => {
                    return AppError::Internal(format!("transaction conflict: {}", db.message()));
                }
                _ => {}
            }
        }
        AppError::Database(e)
    }
}

impl From<yahoo::YahooError> for AppError {
    fn from(e: yahoo::YahooError) -> Self { AppError::Yahoo(e) }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Validation(format!("invalid params: {e}"))
    }
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(e: tower_sessions::session::Error) -> Self {
        AppError::Internal(format!("session error: {e}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Internal(format!("http client error: {e}"))
    }
}


impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "You are forbidden".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "You are unauthorized".to_string()),
            AppError::Database(err) => {
                tracing::error!("database error: {err}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            },
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            },
             AppError::Yahoo(err) => {
                tracing::error!("yahoo api error: {err}");
                (StatusCode::BAD_GATEWAY, "Failed to fetch market data".to_string())
            },

        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub inner: Arc<InnerState>,
}

#[derive(Debug, Clone)]
pub struct InnerState {
    pub public_string: String,
    pub db: PgPool,
}

impl AppState{
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;    

        let inner = InnerState {
            public_string: "Hello, world!".to_string(),
            db,
        };
        Ok(
            AppState {
                inner: Arc::new(inner),
            }
        )
    }
}

