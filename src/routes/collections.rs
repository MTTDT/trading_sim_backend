use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;
use utoipa;

use crate::collection::{Collection, CollectionName};
use crate::http_util::{deleted, ok_or_404};
use crate::state::{AppError, AppState};
use crate::strategy::Strategy;

#[utoipa::path(
    post, path = "/strategies/{sid}/collections",
    params(("sid" = Uuid, Path, description = "Strategy ID")),
    request_body = CollectionName,
    responses(
        (status = 201, description = "Collection created", body = Collection),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Strategy not found"),
        (status = 409, description = "A collection with this name already exists in the strategy"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Missing or invalid name")
    ),
    tag = "collections"
)]pub async fn create_single(
    State(state): State<AppState>,
    Path(sid): Path<Uuid>,
    Json(payload): Json<CollectionName>,
) -> Result<Response, AppError> {
    if Strategy::get_by_id(&state.inner.db, sid).await?.is_none() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let c = Collection::create(&state.inner.db, sid, &payload.name).await?;
    Ok((StatusCode::CREATED, Json(c)).into_response())
}

#[utoipa::path(
    get, path = "/strategies/{sid}/collections",
    params(("sid" = Uuid, Path, description = "Strategy ID")),
    responses(
        (status = 200, description = "Collections of the strategy", body = [Collection]),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Strategy not found")
    ),
    tag = "collections"
)]pub async fn get_all(
    State(state): State<AppState>,
    Path(sid): Path<Uuid>,
) -> Result<Response, AppError> {
    if Strategy::get_by_id(&state.inner.db, sid).await?.is_none() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let list = Collection::get_for_strategy(&state.inner.db, sid).await?;
    Ok((StatusCode::OK, Json(list)).into_response())
}

#[utoipa::path(
    get, path = "/strategies/{sid}/collections/{cid}",
    params(
        ("sid" = Uuid, Path, description = "Strategy ID"),
        ("cid" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 200, description = "Collection found", body = Collection),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Collection not found in this strategy")
    ),
    tag = "collections"
)]

pub async fn get_one(
    State(state): State<AppState>,
    Path((sid, cid)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Collection::get_in_strategy(&state.inner.db, sid, cid).await?))
}

#[utoipa::path(
    put, path = "/strategies/{sid}/collections/{cid}",
    params(
        ("sid" = Uuid, Path, description = "Strategy ID"),
        ("cid" = Uuid, Path, description = "Collection ID")
    ),
    request_body = CollectionName,
    responses(
        (status = 200, description = "Collection renamed", body = Collection),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "Collection not found in this strategy"),
        (status = 409, description = "Name already used in the strategy"),
        (status = 422, description = "Missing or invalid name")
    ),
    tag = "collections"
)]
pub async fn update(
    State(state): State<AppState>,
    Path((sid, cid)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CollectionName>,
) -> Result<Response, AppError> {
    Ok(ok_or_404(Collection::rename(&state.inner.db, sid, cid, &payload.name).await?))
}

#[utoipa::path(
    delete, path = "/strategies/{sid}/collections/{cid}",
    params(
        ("sid" = Uuid, Path, description = "Strategy ID"),
        ("cid" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 204, description = "Collection deleted (its holdings are removed with it)"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Collection not found in this strategy")
    ),
    tag = "collections"
)]
pub async fn delete(
    State(state): State<AppState>,
    Path((sid, cid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    Ok(deleted(Collection::delete(&state.inner.db, sid, cid).await?))
}