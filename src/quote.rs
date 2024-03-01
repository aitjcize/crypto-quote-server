use std::error::Error;
use std::io::{Error as IoError, ErrorKind};

use reqwest::StatusCode;
use rust_decimal::prelude::Decimal;
use serde_json::Value;

async fn fetch_price_from_coingecko(ids: &[&str]) -> Result<Vec<Decimal>, Box<dyn Error>> {
    const COINGECKO_API_TMPL: &str =
        "https://api.coingecko.com/api/v3/simple/price?vs_currencies=USD&ids=";

    let client = reqwest::Client::new();
    let resp = client
            .get(format!("{}{}", COINGECKO_API_TMPL, ids.join(",")))
            .timeout(std::time::Duration::from_secs(5))
            .send().await?;

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
        let mut result = Decimal::from_str_radix(&value[id]["usd"].to_string(), 10);
        if result.is_err() {
            result = Decimal::from_scientific(&value[id]["usd"].to_string());
        }
        result.unwrap()
    }).collect();

    Ok(result)
}


pub async fn get_price(ids: &[&str]) -> Result<Vec<Decimal>, Box<dyn Error>> {
    Ok(fetch_price_from_coingecko(ids).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        let prices = get_price(&["bitcoin", "ethereum", "shiba-inu"]).await.unwrap();
        assert_eq!(prices.len(), 3);
        assert!(prices[0] > Decimal::new(10000, 0));
        assert!(prices[1] > Decimal::new(1000, 0));
        assert!(prices[2] > Decimal::new(0, 0));
    }
}
