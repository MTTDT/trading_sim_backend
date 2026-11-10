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
use crate::state::{AppError, AppState};
use crate::stock::{NewStock, Stock, UpdateStock};

#[derive(Deserialize, IntoParams)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[utoipa::path(
    post, path = "/stocks",
    request_body = NewStock,
    responses(
        (status = 201, description = "Stock created", body = Stock),
        (status = 400, description = "Malformed JSON"),
        (status = 409, description = "Stock with this symbol and exchange already exists"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Missing or invalid fields")
    ),
    tag = "stocks"
)]
pub async fn create_single(
    State(state): State<AppState>,
    Json(payload): Json<NewStock>,
) -> Result<(StatusCode, Json<Stock>), AppError> {
    let stock = Stock::create(&state.inner.db, &payload).await?;
    Ok((StatusCode::CREATED, Json(stock)))
}
#[utoipa::path(
    get, path = "/stocks",
    params(SearchQuery),
    responses(
        (status = 200, description = "List of stocks", body = [Stock]),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "stocks"
)]
pub async fn get_all(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<(StatusCode, Json<Vec<Stock>>), AppError> {
    let list = Stock::get_all(&state.inner.db, params.q.as_deref()).await?;
    Ok((StatusCode::OK, Json(list)))
}

#[utoipa::path(
    get, path = "/stocks/{id}",
    params(("id" = Uuid, Path, description = "Stock ID")),
    responses(
        (status = 200, description = "Stock found", body = Stock),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Stock not found")
    ),
    tag = "stocks"
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Stock::get_by_id(&state.inner.db, id).await?))
}

#[utoipa::path(
    put, path = "/stocks/{id}",
    params(("id" = Uuid, Path, description = "Stock ID")),
    request_body = UpdateStock,
    responses(
        (status = 200, description = "Stock updated", body = Stock),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Stock not found"),
        (status = 409, description = "Symbol and exchange already used by another stock"),
        (status = 422, description = "Invalid fields")
    ),
    tag = "stocks"
)]

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStock>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Stock::update(&state.inner.db, id, &payload).await?))
}

#[utoipa::path(
    delete, path = "/stocks/{id}",
    params(("id" = Uuid, Path, description = "Stock ID")),
    responses(
        (status = 204, description = "Stock deleted"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Stock not found"),
        (status = 409, description = "Stock is still held in a collection")
    ),
    tag = "stocks"
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Ok(deleted(Stock::delete(&state.inner.db, id).await?))
}