#![feature(async_closure)]
#[macro_use]
extern crate rocket;
extern crate dotenv;

use std::env;
use std::time::Duration;

use dotenv::dotenv;
use rocket::http::Status;
use rocket::response::content::Xml;
use rocket::State;
use rust_decimal::prelude::Decimal;
use serde::Serialize;
use serde_xml_rs::to_string;

mod cache;
mod quote;
mod wallet;

struct Cacher {
    price_cache: cache::TimedMemCached<String, Decimal>,
}

impl Cacher {
    fn new(expires: Duration) -> Cacher {
        Cacher {
            price_cache: cache::TimedMemCached::new(expires, Box::new(quote::get_price)),
        }
    }
}

#[get("/quote?<id>")]
async fn quote_handler(cacher: &State<Cacher>, id: &str) -> Xml<(Status, String)> {
    #[derive(Debug, Serialize)]
    struct Quote<'a> {
        id: &'a str,
        price: &'a str,
    }

    match cacher.price_cache.get(id.to_string()).await {
        Ok(price) => {
            let price = price.to_string();
            let quote = Quote {
                id,
                price: price.as_str(),
            };
            Xml((Status::Ok, to_string(&quote).unwrap()))
        }
        Err(e) => Xml((Status::BadRequest, e.to_string())),
    }
}

#[get("/wallet_balance?<chain_id>&<token>&<address>")]
async fn wallet_balance_handler(
    _cacher: &State<Cacher>,
    chain_id: &str,
    token: Option<&str>,
    address: &str,
) -> Xml<(Status, String)> {
    #[derive(Debug, Serialize)]
    struct WalletBalance<'a> {
        chain_id: &'a str,
        token: Option<&'a str>,
        address: &'a str,
        balance: &'a str,
    }
    match wallet::get_balance(chain_id, token, address).await {
        Ok(balance) => {
            let balance = balance.to_string();
            let wallet_balance = WalletBalance {
                chain_id,
                token,
                address,
                balance: balance.as_str(),
            };
            Xml((Status::Ok, to_string(&wallet_balance).unwrap()))
        }
        Err(e) => Xml((Status::BadRequest, e.to_string())),
    }
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let cache_secs: u64 = env::var("CACHE_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap();

    rocket::build()
        .manage(Cacher::new(Duration::from_secs(cache_secs)))
        .mount("/", routes![quote_handler, wallet_balance_handler])
}
