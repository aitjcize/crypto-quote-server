#![feature(async_closure)]
#[macro_use]
extern crate rocket;

use std::time::Duration;

use rocket::http::Status;
use rocket::response::content::Xml;
use rocket::State;
use rust_decimal::prelude::Decimal;
use serde::Serialize;
use serde_xml_rs::to_string;

mod cache;
mod quote;

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

#[derive(Debug, Serialize)]
struct Quote {
    id: String,
    price: String,
}

#[get("/quote?<id>")]
async fn quote_handler(cacher: &State<Cacher>, id: &str) -> Xml<(Status, String)> {
    match cacher.price_cache.get(id.to_string()).await {
        Ok(price) => {
            let quote = Quote {
                id: id.to_string(),
                price: price.to_string(),
            };
            Xml((Status::Ok, to_string(&quote).unwrap()))
        }
        Err(e) => Xml((Status::BadRequest, e.to_string())),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Cacher::new(Duration::from_secs(300)))
        .mount("/", routes![quote_handler])
}
