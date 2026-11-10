mod state;
mod collection;
mod stock;
mod user;
mod strategy;
mod market_data;
mod routes;
mod indicator;
mod http_util;
mod strategy_indicator;
mod holding;
mod filter_request;
mod session_store;
use std::env;
use state::AppState;
use crate::{routes::build_router, user::User};


#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()),
    )
    .init();

    dotenvy::dotenv().expect("failed to load .env file");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    println!("Connecting to: {db_url}");
    
    let state = match AppState::new(&db_url).await {
        Ok(state) => state,
        Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {e:?}"),
    };

    let session_layer = routes::auth::session_layer(state.inner.db.clone())
        .await
        .expect("failed to set up session store");


    let router = build_router(state).route_layer(session_layer);

    

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid port number");

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();

}
