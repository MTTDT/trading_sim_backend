use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::collection::Collection;
use crate::holding::{Holding, NewHolding, UpdateHolding};
use crate::http_util::{deleted, ok_or_404};
use crate::state::{AppError, AppState};
use crate::stock::Stock;

async fn parent_exists(state: &AppState, sid: Uuid, cid: Uuid) -> Result<bool, AppError> {
    Ok(Collection::get_in_strategy(&state.inner.db, sid, cid).await?.is_some())
}

pub async fn get_all(
    State(state): State<AppState>,
    Path((sid, cid)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    if !parent_exists(&state, sid, cid).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let list = Holding::list(&state.inner.db, cid).await?;
    Ok((StatusCode::OK, Json(list)).into_response())
}

pub async fn create_single(
    State(state): State<AppState>,
    Path((sid, cid)): Path<(Uuid, Uuid)>,
    Json(payload): Json<NewHolding>,
) -> Result<Response, AppError> {
    if !parent_exists(&state, sid, cid).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    if Stock::get_by_id(&state.inner.db, payload.stock_id).await?.is_none() {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response()); // unknown stock_id in body
    }
    match Holding::add(&state.inner.db, cid, &payload).await? {
        Some(h) => Ok((StatusCode::CREATED, Json(h)).into_response()),
        None => Ok(StatusCode::CONFLICT.into_response()), // already in this collection
    }
}

pub async fn get_one(
    State(state): State<AppState>,
    Path((sid, cid, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Response, AppError> {
    if !parent_exists(&state, sid, cid).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(ok_or_404(Holding::get(&state.inner.db, cid, id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    Path((sid, cid, id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<UpdateHolding>,
) -> Result<Response, AppError> {
    if !parent_exists(&state, sid, cid).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(ok_or_404(Holding::update(&state.inner.db, cid, id, &payload).await?))
}

pub async fn delete(
    State(state): State<AppState>,
    Path((sid, cid, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Response, AppError> {
    if !parent_exists(&state, sid, cid).await? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(deleted(Holding::delete(&state.inner.db, cid, id).await?).into_response())
}