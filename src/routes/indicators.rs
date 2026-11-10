use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::IntoParams;

use crate::http_util::{deleted, ok_or_404};
use crate::indicator::{Indicator, NewIndicator, UpdateIndicator};
use crate::state::{AppError, AppState};


#[derive(Deserialize, IntoParams)]
pub struct ListQuery {
    pub user_id: Option<Uuid>,
}

#[utoipa::path(
    post, path = "/indicators",
    request_body = NewIndicator,
    responses(
        (status = 201, description = "Indicator created", body = Indicator),
        (status = 400, description = "Malformed JSON"),
        (status = 404, description = "Owner user not found"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Missing or invalid fields")
    ),
    tag = "indicators"
)]
pub async fn create_single(
    State(state): State<AppState>,
    Json(payload): Json<NewIndicator>,
) -> Result<(StatusCode, Json<Indicator>), AppError> {
    let indicator = Indicator::create(&state.inner.db, &payload).await?;
    Ok((StatusCode::CREATED, Json(indicator)))
}

#[utoipa::path(
    get, path = "/indicators",
    params(ListQuery),
    responses(
        (status = 200, description = "Built-in indicators plus the user's own", body = [Indicator]),
        (status = 400, description = "Invalid user_id")
    ),
    tag = "indicators"
)]
pub async fn get_all(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<(StatusCode, Json<Vec<Indicator>>), AppError> {
    let list = Indicator::get_visible(&state.inner.db, q.user_id).await?;
    Ok((StatusCode::OK, Json(list)))
}

#[utoipa::path(
    get, path = "/indicators/{id}",
    params(("id" = Uuid, Path, description = "Indicator ID")),
    responses(
        (status = 200, description = "Indicator found", body = Indicator),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Indicator not found")
    ),
    tag = "indicators"
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Indicator::get_by_id(&state.inner.db, id).await?))
}


#[utoipa::path(
    put, path = "/indicators/{id}",
    params(("id" = Uuid, Path, description = "Indicator ID")),
    request_body = UpdateIndicator,
    responses(
        (status = 200, description = "Indicator updated", body = Indicator),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Indicator not found, or it is a built-in (read-only)"),
        (status = 422, description = "Invalid fields")
    ),
    tag = "indicators"
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateIndicator>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Indicator::update(&state.inner.db, id, &payload).await?))
}

#[utoipa::path(
    delete, path = "/indicators/{id}",
    params(("id" = Uuid, Path, description = "Indicator ID")),
    responses(
        (status = 204, description = "Indicator deleted"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Indicator not found, or it is a built-in (read-only)"),
        (status = 409, description = "Indicator is still used by a strategy")
    ),
    tag = "indicators"
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Ok(deleted(Indicator::delete(&state.inner.db, id).await?))
}