use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::time::{Duration, SystemTime};

use reqwest::{self, StatusCode};
use rust_decimal::prelude::Decimal;
use serde_json::{self, Value};
use terra_rust_api::Terra;
use tokio::sync::RwLock;

const COINGECKO_API_TMPL: &str =
    "https://api.coingecko.com/api/v3/simple/price?vs_currencies=USD&ids=";

pub struct PriceCache {
    expires: Duration,
    cache: RwLock<HashMap<String, (SystemTime, Decimal)>>,
}

impl PriceCache {
    pub fn new(expires: u64) -> PriceCache {
        PriceCache {
            expires: Duration::from_secs(expires),
            cache: RwLock::new(HashMap::new()),
        }
    }

    async fn fetch_price_from_coingecko(name: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
        let resp = reqwest::get(format!("{}{}", COINGECKO_API_TMPL, name)).await?;
        if resp.status() != StatusCode::OK {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("HTTP error {}", resp.status()),
            )));
        }

        let value = resp.json::<Value>().await?;
        if let Value::Null = value[name]["usd"] {
            return Err(Box::new(Error::new(ErrorKind::Other, format!("not found"))));
        }
        Ok(Decimal::from_str_radix(
            &value[name]["usd"].to_string(),
            10,
        )?)
    }

    async fn fetch_price_aust() -> Result<Decimal, Box<dyn std::error::Error>> {
        let terra_overseer_address = "terra1tmnqgvg567ypvsvk6rwsga3srp7e3lg6u0elp8";
        let query = "%7B%22epoch_state%22  %3A%7B%7D%7D";

        let client = Terra::lcd_client_no_tx("https://fcd.terra.dev", "columbus-5");
        let value: Value = client.wasm().query(terra_overseer_address, query).await?;

        match &value["result"]["prev_exchange_rate"] {
            Value::String(v) => Ok(Decimal::from_str_radix(&v, 10)?),
            _ => Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("Parse error"),
            ))),
        }
    }

    async fn fetch_price(&self, name: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
        let mut cache = self.cache.write().await;
        // A second check to prevent another writer trying to fetch new price right after previous
        // update due to the write lock.
        if let Some((last_updated, price)) = cache.get(name) {
            if last_updated.elapsed()? < self.expires {
                return Ok(price.clone());
            }
        }

        let price = match name {
            "anchorust" => Self::fetch_price_aust().await?,
            _ => Self::fetch_price_from_coingecko(name).await?,
        };

        cache.insert(name.to_string(), (SystemTime::now(), price.clone()));

        Ok(price)
    }

    pub async fn get_price(&self, name: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
        let cache = self.cache.read().await;
        if let Some((last_updated, price)) = cache.get(name) {
            if last_updated.elapsed()? < self.expires {
                return Ok(price.clone());
            }
        }
        drop(cache);
        self.fetch_price(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_price() {
        let x = PriceCache::new(3);
        let btc_price = x.get_price("bitcoin").await.unwrap();
        assert!(btc_price > Decimal::new(10000, 0));

        let eth_price = x.get_price("ethereum").await.unwrap();
        assert!(eth_price > Decimal::new(1000, 0));

        let eth_price = x.get_price("anchorust").await.unwrap();
        assert!(eth_price > Decimal::new(1, 0));
    }
}
