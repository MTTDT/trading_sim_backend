mod health;
mod users;
mod stock_history;
mod strategies;
mod indicators;
mod strategy_indicators;
mod collections;
mod stocks;
mod holdings;

pub mod auth;
mod oauth;

use crate::state::AppState;


use axum::{routing::{get, post, delete}, Router};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};
#[derive(OpenApi)]
#[openapi(
    paths(
        users::get_all, users::get_one, users::create_single, users::update, users::delete,
        strategies::get_all, strategies::get_all_by_user_id, strategies::create_single,
        strategies::get_one, strategies::update, strategies::delete, strategies::test_strategy,
        indicators::create_single, indicators::get_all, indicators::get_one,
        indicators::update, indicators::delete,
        strategy_indicators::add, strategy_indicators::get_all, strategy_indicators::get_one,
        strategy_indicators::update, strategy_indicators::remove,
        stocks::create_single, stocks::get_all, stocks::get_one,
        stocks::update, stocks::delete,
        collections::create_single, collections::get_all, collections::get_one,
        collections::update, collections::delete,
    ),
    components(schemas(/* ... */)),
    tags(
        (name = "users", description = "Naudotojai"),
        (name = "strategies", description = "Strategijos"),
        (name = "indicators", description = "Built-in and user-defined indicators"),
        (name = "strategy-indicators", description = "Indicators attached to a strategy"),
        (name = "stocks", description = "Tradable stocks"),
        (name = "collections", description = "Stock collections inside a strategy")
    )
)]
pub struct ApiDoc;
pub fn build_router(state: AppState) -> axum::Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/market-data", get(stock_history::get_stock_history))
        
        //users
        .route("/users", get(users::get_all).post(users::create_single))
        .route("/users/{id}", get(users::get_one).put(users::update).delete(users::delete))

        // indicators
        .route("/indicators", get(indicators::get_all).post(indicators::create_single))
        .route("/indicators/{id}",get(indicators::get_one).put(indicators::update).delete(indicators::delete))

        // strategies
        .route("/strategies", get(strategies::get_all).post(strategies::create_single))
        .route("/strategies/{id}", get(strategies::get_one).put(strategies::update).delete(strategies::delete))
        .route("/strategies/{id}/test", post(strategies::test_strategy))
        .route("/users/{id}/strategies", get(strategies::get_all_by_user_id))
     

        // strategy -> indicators
        .route("/strategies/{id}/indicators",get(strategy_indicators::get_all).post(strategy_indicators::add))
        .route("/strategy-indicators/{id}", get(strategy_indicators::get_one).put(strategy_indicators::update).delete(strategy_indicators::remove))

        // strategy -> collections
        .route("/strategies/{sid}/collections",get(collections::get_all).post(collections::create_single))
        .route("/strategies/{sid}/collections/{cid}",get(collections::get_one).put(collections::update).delete(collections::delete))

        // collection -> holdings
        .route("/strategies/{sid}/collections/{cid}/holdings",get(holdings::get_all).post(holdings::create_single))
        .route("/strategies/{sid}/collections/{cid}/holdings/{id}",get(holdings::get_one).put(holdings::update).delete(holdings::delete))

        // stocks
        .route("/stocks", get(stocks::get_all).post(stocks::create_single))
        .route("/stocks/{id}", get(stocks::get_one).put(stocks::update).delete(stocks::delete))

        
        .merge(auth::routes())
        .with_state(state)
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
}