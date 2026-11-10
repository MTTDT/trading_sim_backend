use crate::market_data::{get_history};
use crate::stock::{FullStock};
use crate::state::{AppError};



use axum::extract::Query;
use axum::Json;
use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub symbol: String,
    #[serde(default = "default_time_frame")]
    pub time_frame: String,
    #[serde(default = "default_bar_size")]
    pub bar_size: String,
}

fn default_time_frame() -> String { "6mo".to_string() }
fn default_bar_size() -> String { "1d".to_string() }

pub async fn get_stock_history(
    Query(params): Query<HistoryQuery>,
) -> Result<Json<FullStock>, AppError> {
    let stock = get_history(&params.symbol, &params.time_frame, &params.bar_size).await?;
    Ok(Json(stock))
}