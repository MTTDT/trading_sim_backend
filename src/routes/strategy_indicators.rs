use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use uuid::Uuid;

use crate::http_util::{deleted, ok_or_404};
use crate::state::{AppError, AppState};
use crate::strategy_indicator::{NewStrategyIndicator, StrategyIndicator, UpdateStrategyIndicator};

#[utoipa::path(
    post, path = "/strategies/{id}/indicators",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    request_body = NewStrategyIndicator,
    responses(
        (status = 201, description = "Indicator attached to the strategy", body = StrategyIndicator),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Strategy or indicator not found"),
        (status = 409, description = "Indicator already attached"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Invalid role, weight, params or position")
    ),
    tag = "strategy-indicators"
)]
pub async fn add(
    State(state): State<AppState>,
    Path(strategy_id): Path<Uuid>,
    Json(payload): Json<NewStrategyIndicator>,
) -> Result<(StatusCode, Json<StrategyIndicator>), AppError> {
    let link = StrategyIndicator::add(&state.inner.db, strategy_id, &payload).await?;
    Ok((StatusCode::CREATED, Json(link)))
}

#[utoipa::path(
    get, path = "/strategies/{id}/indicators",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    responses(
        (status = 200, description = "Indicators attached to the strategy", body = [StrategyIndicator]),
        (status = 400, description = "Invalid UUID")
    ),
    tag = "strategy-indicators"
)]
pub async fn get_all(
    State(state): State<AppState>,
    Path(strategy_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Vec<StrategyIndicator>>), AppError> {
    let list = StrategyIndicator::get_for_strategy(&state.inner.db, strategy_id).await?;
    Ok((StatusCode::OK, Json(list)))
}

#[utoipa::path(
    get, path = "/strategy-indicators/{id}",
    params(("id" = Uuid, Path, description = "Strategy-indicator link ID")),
    responses(
        (status = 200, description = "Link found", body = StrategyIndicator),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Link not found")
    ),
    tag = "strategy-indicators"
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(StrategyIndicator::get_by_id(&state.inner.db, id).await?))
}

#[utoipa::path(
    put, path = "/strategy-indicators/{id}",
    params(("id" = Uuid, Path, description = "Strategy-indicator link ID")),
    request_body = UpdateStrategyIndicator,
    responses(
        (status = 200, description = "Link updated", body = StrategyIndicator),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Link not found"),
        (status = 422, description = "Invalid role, weight, params or position")
    ),
    tag = "strategy-indicators"
)]

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStrategyIndicator>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(StrategyIndicator::update(&state.inner.db, id, &payload).await?))
}

#[utoipa::path(
    delete, path = "/strategy-indicators/{id}",
    params(("id" = Uuid, Path, description = "Strategy-indicator link ID")),
    responses(
        (status = 204, description = "Link removed"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Link not found")
    ),
    tag = "strategy-indicators"
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Ok(deleted(StrategyIndicator::remove(&state.inner.db, id).await?))
}