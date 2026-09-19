use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use crate::state::AppError;
pub async fn get() -> Result<(StatusCode, &'static str), AppError>{
    Ok((StatusCode::OK, "User route is working!"))

}