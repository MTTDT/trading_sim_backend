use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub fn ok_or_404<T: Serialize>(value: Option<T>) -> Response {
    match value {
        Some(v) => (StatusCode::OK, Json(v)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn deleted(was_deleted: bool) -> StatusCode {
    if was_deleted {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}