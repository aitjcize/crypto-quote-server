#![feature(async_closure)]
#[macro_use]
extern crate rocket;
extern crate dotenv;

use dotenv::dotenv;
use rocket::http::Status;
use rocket::response::content::RawXml;
use serde::Serialize;
use quick_xml::se::to_string;

mod quote;
mod wallet;


#[derive(Debug, Serialize, PartialEq)]
struct Error<'a> {
    response: &'a str
}

#[get("/alive")]
async fn index() -> &'static str {
    "OK"
}

#[get("/quote?<ids>")]
async fn quote_handler(ids: &str) -> RawXml<(Status, String)> {
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
            RawXml((Status::Ok, to_string(&quotes).unwrap()))
        }
        Err(e) => {
            let error = Error {
                response: &e.to_string()
            };
            RawXml((Status::BadRequest, to_string(&error).unwrap()))
        }
    }
}

#[get("/wallet_balance?<chain_id>&<token>&<address>")]
async fn wallet_balance_handler(
    chain_id: &str,
    token: Option<&str>,
    address: &str,
) -> RawXml<(Status, String)> {
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
            RawXml((Status::Ok, to_string(&wallet_balance).unwrap()))
        }
        Err(e) => {
            let error = Error {
                response: &e.to_string()
            };
            RawXml((Status::BadRequest, to_string(&error).unwrap()))
        }
    }
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build().mount("/", routes![index, quote_handler, wallet_balance_handler])
}
