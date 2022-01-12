use std::io::{Error, ErrorKind};

use reqwest::{self, StatusCode};
use rust_decimal::prelude::Decimal;
use serde_json::{self, Value};
use terra_rust_api::Terra;

async fn fetch_price_from_coingecko(id: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
    const COINGECKO_API_TMPL: &str =
        "https://api.coingecko.com/api/v3/simple/price?vs_currencies=USD&ids=";

    let resp = reqwest::get(format!("{}{}", COINGECKO_API_TMPL, id)).await?;
    if resp.status() != StatusCode::OK {
        return Err(Box::new(Error::new(
            ErrorKind::Other,
            format!("HTTP error {}", resp.status()),
        )));
    }

    let value = resp.json::<Value>().await?;
    if let Value::Null = value[id]["usd"] {
        return Err(Box::new(Error::new(ErrorKind::Other, format!("id not found"))));
    }
    Ok(Decimal::from_str_radix(
        &value[id]["usd"].to_string(),
        10,
    )?)
}

async fn fetch_price_aust() -> Result<Decimal, Box<dyn std::error::Error>> {
    let anchor_overseer_address = "terra1tmnqgvg567ypvsvk6rwsga3srp7e3lg6u0elp8";
    let query = "%7B%22epoch_state%22  %3A%7B%7D%7D";

    let client = Terra::lcd_client_no_tx("https://fcd.terra.dev", "columbus-5");
    let value: Value = client.wasm().query(anchor_overseer_address, query).await?;

    match &value["result"]["prev_exchange_rate"] {
        Value::String(v) => Ok(Decimal::from_str_radix(&v, 10)?),
        _ => Err(Box::new(Error::new(
            ErrorKind::Other,
            format!("Parse error"),
        ))),
    }
}

pub async fn get_price(id: String) -> Result<Decimal, Box<dyn std::error::Error>> {
    let price = match id.as_str() {
        "anchorust" => fetch_price_aust().await?,
        _ => fetch_price_from_coingecko(id.as_str()).await?,
    };
    println!("fetch!!!!");
    Ok(price)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        let btc_price = get_price("bitcoin".to_string()).await.unwrap();
        assert!(btc_price > Decimal::new(10000, 0));

        let eth_price = get_price("ethereum".to_string()).await.unwrap();
        assert!(eth_price > Decimal::new(1000, 0));

        let eth_price = get_price("anchorust".to_string()).await.unwrap();
        assert!(eth_price > Decimal::new(1, 0));
    }
}
