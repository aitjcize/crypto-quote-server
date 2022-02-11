use std::error::Error;
use std::io::{Error as IoError, ErrorKind};

use reqwest::{self, StatusCode};
use rust_decimal::prelude::Decimal;
use serde_json::{self, Value};
use terra_rust_api::Terra;

async fn fetch_price_from_coingecko(ids: &[&str]) -> Result<Vec<Decimal>, Box<dyn Error>> {
    const COINGECKO_API_TMPL: &str =
        "https://api.coingecko.com/api/v3/simple/price?vs_currencies=USD&ids=";

    let resp = reqwest::get(format!("{}{}", COINGECKO_API_TMPL, ids.join(","))).await?;
    if resp.status() != StatusCode::OK {
        return Err(Box::new(IoError::new(
            ErrorKind::Other,
            format!("HTTP error {}", resp.status()),
        )));
    }

    let value = resp.json::<Value>().await?;
    let result : Vec<Decimal> = ids.iter().map(|id| {
        if let Value::Null = value[id]["usd"] {
            return Decimal::new(0, 0);
        }
        Decimal::from_str_radix(&value[id]["usd"].to_string(), 10).unwrap()
    }).collect();

    Ok(result)
}

async fn fetch_price_aust() -> Result<Decimal, Box<dyn Error>> {
    let anchor_overseer_address = "terra1tmnqgvg567ypvsvk6rwsga3srp7e3lg6u0elp8";
    let query = "%7B%22epoch_state%22%3A%7B%7D%7D";

    let client = Terra::lcd_client_no_tx("https://fcd.terra.dev", "columbus-5");
    let value: Value = client.wasm().query(anchor_overseer_address, query).await?;

    match &value["result"]["prev_exchange_rate"] {
        Value::String(v) => Ok(Decimal::from_str_radix(v, 10)?),
        _ => Err(Box::new(IoError::new(
            ErrorKind::Other,
            "Parse error".to_string(),
        ))),
    }
}

pub async fn get_price(ids: &[&str]) -> Result<Vec<Decimal>, Box<dyn Error>> {
    let mut result = fetch_price_from_coingecko(ids).await?;

    // Query accurate anchor vprice from contract.
    if let Some(idx) = ids.iter().position(|id| (*id).eq("anchorust")) {
        result[idx] = fetch_price_aust().await?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        let prices = get_price(&["bitcoin", "ethereum", "anchorust"]).await.unwrap();
        assert_eq!(prices.len(), 3);
        assert!(prices[0] > Decimal::new(10000, 0));
        assert!(prices[1] > Decimal::new(1000, 0));
        assert!(prices[2] > Decimal::new(0, 0));
    }
}
