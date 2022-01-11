#[macro_use] extern crate rocket;

use rocket::http::Status;
use rocket::response::content::Xml;
use serde::Serialize;
use serde_xml_rs::to_string;

mod quote;

#[derive(Debug, Serialize)]
struct Quote {
    id: String,
    price: String,
}

#[get("/quote?<id>")]
async fn quote_handler(id: &str) -> Xml<(Status, String)> {
    match quote::get_price(&id).await {
        Ok(price) => {
            let quote = Quote {
                id: id.to_string(),
                price: price.to_string(),
            };
            Xml((Status::Ok, to_string(&quote).unwrap()))
        }
        Err(e) => {
            Xml((Status::BadRequest, e.to_string()))
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![quote_handler])
}
