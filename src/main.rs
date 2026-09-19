mod state;
mod routes;

use std::env;
use state::AppState;
use crate::routes::build_router;


#[tokio::main]
async fn main() {

    dotenvy::dotenv().expect("failed to load .env file");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    
    let state = match AppState::new(&db_url).await {
        Ok(state) => state,
        Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {e:?}"),
    };


    let router = build_router(state);

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
