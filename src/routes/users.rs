use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::{AppError, AppState};
use crate::user::User;
use crate::filter_request::FilterRequest;

use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    /// Display name
    #[schema(example = "Jonas Jonaitis")]
    pub name: String,
    /// Unique email address
    #[schema(example = "jonas@example.com")]
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = User),
        (status = 400, description = "Malformed JSON"),
        (status = 409, description = "Email already exists"),
        (status = 415, description = "Content-Type is not application/json"),
        (status = 422, description = "Missing or invalid fields")
    ),
    tag = "users"
)]
pub async fn create_single(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = User::create(&state.inner.db, &payload.name, &payload.email).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

#[utoipa::path(
    get,
    path = "/users",
    params(FilterRequest),
    responses(
        (status = 200, description = "List of users", body = [User]),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "users"
)]
pub async fn get_all(
    State(state): State<AppState>,
    Query(f_req): Query<FilterRequest>,
) -> Result<(StatusCode, Json<Vec<User>>), AppError> {
    let users = User::get_all(&state.inner.db, f_req).await?;
    Ok((StatusCode::OK, Json(users)))
}

#[utoipa::path(
    get,
    path = "/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "User not found")
    ),
    tag = "users"
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    match User::get_by_id(&state.inner.db, id).await? {
        Some(user) => Ok((StatusCode::OK, Json(user)).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = User),
        (status = 400, description = "Invalid UUID or malformed JSON"),
        (status = 404, description = "User not found"),
        (status = 409, description = "Email already exists"),
        (status = 422, description = "Invalid fields")
    ),
    tag = "users"
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Response, AppError> {
    let updated = User::update(
        &state.inner.db,
        id,
        payload.name.as_deref(),
        payload.email.as_deref(),
    )
    .await?;

    match updated {
        Some(user) => Ok((StatusCode::OK, Json(user)).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "User not found"),
        (status = 409, description = "User still owns strategies or indicators")
    ),
    tag = "users"
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if User::delete(&state.inner.db, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}