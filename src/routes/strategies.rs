use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::filter_request::FilterRequest;
use crate::http_util::{deleted, ok_or_404};
use crate::state::{AppError, AppState};
use crate::strategy::{NewStrategy, Strategy, TestParams, UpdateStrategy, TestResult};

#[derive(Deserialize)]
pub struct ListQuery {
    pub user_id: Option<Uuid>,
}

#[utoipa::path(
    post, path = "/strategies",
    request_body = NewStrategy,
    responses(
        (status = 201, description = "Strategy created", body = Strategy),
        (status = 400, description = "Malformed JSON"),
        (status = 404, description = "Owner user not found"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Validation failed")
    ),
    tag = "strategies"
)]
pub async fn create_single(
    State(state): State<AppState>,
    Json(payload): Json<NewStrategy>,
) -> Result<(StatusCode, Json<Strategy>), AppError> {
    payload.validate()?;                      // <- 422 before touching the DB
    let strategy = Strategy::create(&state.inner.db, &payload).await?;
    Ok((StatusCode::CREATED, Json(strategy)))
}

#[utoipa::path(
    get, path = "/strategies",
    params(FilterRequest),
    responses(
        (status = 200, description = "Public strategies", body = [Strategy]),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "strategies"
)]
pub async fn get_all(
    State(state): State<AppState>,
    Query(f_req): Query<FilterRequest>,
) -> Result<(StatusCode, Json<Vec<Strategy>>), AppError> {
    let list = Strategy::get_all(&state.inner.db, f_req).await?;
    Ok((StatusCode::OK, Json(list)))
}

#[utoipa::path(
    get, path = "/users/{id}/strategies",
    params(("id" = Uuid, Path, description = "User ID"), FilterRequest),
    responses(
        (status = 200, description = "Strategies owned by the user", body = [Strategy]),
        (status = 400, description = "Invalid UUID or query parameters")
    ),
    tag = "strategies"
)]
pub async fn get_all_by_user_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(f_req): Query<FilterRequest>,
) -> Result<(StatusCode, Json<Vec<Strategy>>), AppError> {
    
    let list = Strategy::get_all_by_user_id(&state.inner.db, id, f_req).await?;
    Ok((StatusCode::OK, Json(list)))
}



#[utoipa::path(
    get, path = "/strategies/{id}",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    responses(
        (status = 200, description = "Strategy found", body = Strategy),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Strategy not found")
    ),
    tag = "strategies"
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Strategy::get_by_id(&state.inner.db, id).await?))
}

#[utoipa::path(
    put, path = "/strategies/{id}",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    request_body = UpdateStrategy,
    responses(
        (status = 200, description = "Strategy updated", body = Strategy),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Strategy not found"),
        (status = 422, description = "Validation failed")
    ),
    tag = "strategies"
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStrategy>,
) -> Result<Response, AppError> {
    payload.validate()?;

    let existing = Strategy::get_by_id(&state.inner.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
        // if existing.is_builtin {
        //     return Err(AppError::Forbidden);      
        // }

    let updated = Strategy::update(&state.inner.db, id, &payload)
        .await?
        .ok_or(AppError::NotFound)?;          // guards against a concurrent delete
    Ok((StatusCode::OK, Json(updated)).into_response())
}
#[utoipa::path(
    delete, path = "/strategies/{id}",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    responses(
        (status = 204, description = "Strategy deleted"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Strategy not found")
    ),
    tag = "strategies"
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Ok(deleted(Strategy::delete(&state.inner.db, id).await?))
}

#[utoipa::path(
    post, path = "/strategies/{id}/test",
    params(("id" = Uuid, Path, description = "Strategy ID")),
    request_body = TestParams,
    responses(
        (status = 200, description = "Backtest result", body = TestResult),
        (status = 400, description = "Invalid UUID, malformed JSON or bad time_frame/bar_size"),
        (status = 404, description = "Strategy not found or no market data"),
        (status = 422, description = "Strategy cannot be tested (no stocks, no entry indicator, unsupported indicator)")
    ),
    tag = "strategies"
)]
pub async fn test_strategy(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TestParams>,
) -> Result<Response, AppError> {
    let strategy = Strategy::get_by_id(&state.inner.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let result = strategy.test_strategy(&state.inner.db, &payload).await?;
    Ok((StatusCode::OK, Json(result)).into_response())
}