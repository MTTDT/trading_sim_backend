
use crate::stock::{FullStock};
use crate::state::{AppError};
use yahoo_finance_api as yahoo;
use time::{OffsetDateTime, macros::datetime};

pub async fn get_history(symbol: &str, time_frame: &str, bar_size: &str) -> Result<FullStock, AppError> {
    let provider = yahoo::YahooConnector::new()
        .map_err(|e| AppError::Internal(format!("failed to create yahoo connector: {e}")))?;;

    let response = provider
        .get_quote_range(symbol, bar_size, time_frame)
        .await
        .map_err(AppError::Yahoo)?;

    let quotes = response.quotes().map_err(AppError::Yahoo)?;

     if quotes.is_empty() {
        return Err(AppError::NotFound);
    }

    let (open_prices, close_prices): (Vec<f64>, Vec<f64>) =
        quotes.iter().map(|q| (q.open, q.close)).unzip();


    Ok(FullStock {
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            close_prices,
            open_prices,
        })
}