mod health;
mod users;

use crate::state::AppState;
use axum::{routing::{get, post}, Router};

pub fn build_router(state: AppState) -> axum::Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/users", get(users::get))
        .with_state(state)
}