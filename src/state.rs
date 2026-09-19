use std::sync::Arc;
use sqlx::postgres::{PgPool, PgPoolOptions};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    NotFound,
    Database(sqlx::Error),
    Validation(String),
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Database(other),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::Database(err) => {
                tracing::error!("database error: {err}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            },
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
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

