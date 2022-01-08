use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use clap::Parser;
use crypto_quote_server::PriceCache;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
use serde_derive::{Deserialize, Serialize};
use serde_xml_rs::to_string;
use url;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long, default_value_t = 300)]
    expires: u64,

    #[clap(short, long, default_value = "0.0.0.0")]
    host: Ipv4Addr,

    #[clap(short, long, default_value_t = 8080)]
    port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct Quote {
    name: String,
    price: String,
}

async fn handle(
    req: Request<Body>,
    addr: SocketAddr,
    price_cache: Arc<PriceCache>,
) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/quote") => {
            let params: HashMap<String, String> = req
                .uri()
                .query()
                .map(|v| {
                    url::form_urlencoded::parse(v.as_bytes())
                        .into_owned()
                        .collect()
                })
                .unwrap_or_else(HashMap::new);

            match params.get("name") {
                Some(name) => match price_cache.get_price(&name).await {
                    Ok(price) => {
                        let quote = Quote {
                            name: name.to_string(),
                            price: price.to_string(),
                        };
                        *response.body_mut() = Body::from(to_string(&quote).unwrap());
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            header::HeaderValue::from_static("application/xml"),
                        );
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        *response.body_mut() = Body::from(e.to_string());
                        *response.status_mut() = StatusCode::BAD_REQUEST;
                    }
                },
                None => {
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                }
            }
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    println!(
        "{} - - \"{} {}\" {} -",
        addr.ip(),
        req.method(),
        req.uri(),
        response.status()
    );

    Ok(response)
}

#[tokio::main]
pub async fn main() {
    let args = Args::parse();

    pretty_env_logger::init();

    let price_cache = Arc::new(PriceCache::new(args.expires));

    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let addr = conn.remote_addr();
        let price_cache = Arc::clone(&price_cache);

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle(req, addr, Arc::clone(&price_cache))
            }))
        }
    });

    let addr = (args.host, args.port).into();
    let server = Server::bind(&addr).serve(make_svc);

    println!("Crypto Quote Server started.");
    println!("Cache expiration: {}s", args.expires);
    println!("Listening on http://{} ...", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
