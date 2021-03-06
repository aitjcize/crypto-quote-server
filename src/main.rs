#![feature(async_closure)]
#[macro_use]
extern crate rocket;
extern crate dotenv;

use dotenv::dotenv;
use rocket::http::Status;
use rocket::response::content::Xml;
use serde::Serialize;
use quick_xml::se::to_string;

mod quote;
mod wallet;

#[get("/quote?<ids>")]
async fn quote_handler(ids: &str) -> Xml<(Status, String)> {
    #[derive(Debug, Serialize, PartialEq)]
    struct Price {
        id: String,
        #[serde(rename = "$value")]
        price: String,
    }

    #[derive(Debug, Serialize, PartialEq)]
    struct Quotes {
        #[serde(rename = "price")]
        quotes: Vec<Price>,
    }

    let ids_vec: Vec<&str> = ids.split(',').collect();

    match quote::get_price(&ids_vec).await {
        Ok(prices) => {
            let quotes = Quotes {
                quotes: ids_vec
                    .iter()
                    .zip(prices.iter())
                    .map(|(id, price)| Price {
                        id: id.to_string(),
                        price: price.to_string(),
                    })
                    .collect(),
            };
            Xml((Status::Ok, to_string(&quotes).unwrap()))
        }
        Err(e) => Xml((Status::BadRequest, e.to_string())),
    }
}

#[get("/wallet_balance?<chain_id>&<token>&<address>")]
async fn wallet_balance_handler(
    chain_id: &str,
    token: Option<&str>,
    address: &str,
) -> Xml<(Status, String)> {
    #[derive(Debug, Serialize)]
    struct WalletBalance<'a> {
        chain_id: &'a str,
        token: Option<&'a str>,
        address: &'a str,
        #[serde(rename = "$value")]
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

    rocket::build().mount("/", routes![quote_handler, wallet_balance_handler])
}
