use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::{Cursor, Error as IoError, ErrorKind};

use ethcontract::contract::ViewMethodBuilder;
use ethcontract::Instance;
use ethcontract_common::{hash, Abi};
use num_traits::Pow;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;
use serde_json::{self, Value};
use terra_rust_api::Terra;
use web3::types::{Address, U256};
use web3::Web3;

const ERC20_ABI: &str = include_str!("erc20.json");

fn get_web3_transport(chain_id: &str) -> Result<Web3<web3::transports::Http>, Box<dyn Error>> {
    let rpc_endpoint: String = match chain_id {
        "ethereum" => {
            let token = env::var("INFURA_TOKEN")?;
            Ok(format!("https://mainnet.infura.io/v3/{}", token))
        }
        "polygon" => Ok("https://polygon-rpc.com".into()),
        "avalanche" => Ok("https://api.avax.network/ext/bc/C/rpc".into()),
        "moonbeam" => Ok("https://rpc.api.moonbeam.network".into()),
        "bsc" => Ok("https://bsc-dataseed1.ninicoin.io".into()),
        _ => Err(IoError::new(
            ErrorKind::Other,
            "invalid chain_id id".to_string(),
        )),
    }?;
    let transport = web3::transports::Http::new(&rpc_endpoint)?;
    Ok(web3::Web3::new(transport))
}

async fn web3_get_balance(chain_id: &str, address: &str) -> Result<Decimal, Box<dyn Error>> {
    let web3 = get_web3_transport(chain_id)?;
    let balance: Decimal = web3
        .eth()
        .balance(address.parse()?, None)
        .await?
        .to_string()
        .parse()?;
    let decimals = Pow::pow(Decimal::new(10, 0), 18_u64);
    Ok(balance / decimals)
}

async fn web3_get_erc20_token_balance(
    chain_id: &str,
    contract_address: &str,
    address: &str,
) -> Result<Decimal, Box<dyn Error>> {
    let web3 = get_web3_transport(chain_id)?;
    let f = Cursor::new(ERC20_ABI.as_bytes().to_vec());
    let instance = Instance::at(web3, Abi::load(f)?, contract_address.parse()?);
    let address: Address = address.parse()?;
    let v: ViewMethodBuilder<_, u8> =
        instance.view_method(hash::function_selector("decimals()"), ())?;
    let decimals: u8 = v.call().await?;
    let decimals = Pow::pow(Decimal::new(10, 0), decimals as u64);

    let v: ViewMethodBuilder<_, U256> =
        instance.view_method(hash::function_selector("balanceOf(address)"), (address,))?;
    let balance: Decimal = v.call().await?.to_string().parse()?;

    Ok(balance / decimals)
}

async fn polkadot_get_balance(address: &str) -> Result<Decimal, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut map = HashMap::new();
    map.insert("address", address.to_string());
    let resp = client
        .post("https://polkadot.api.subscan.io/api/open/account")
        .json(&map)
        .send()
        .await?;

    #[derive(Deserialize)]
    struct Data {
        balance: Decimal,
    }
    #[derive(Deserialize)]
    struct JsonResult {
        data: Data,
    }

    Ok(resp.json::<JsonResult>().await?.data.balance)
}

async fn substrate_get_balance(chain_id: &str, address: &str) -> Result<Decimal, Box<dyn Error>> {
    match chain_id {
        "polkadot" => polkadot_get_balance(address).await,
        _ => Err(Box::new(IoError::new(
            ErrorKind::Other,
            "invalid chain_id".to_string(),
        ))),
    }
}

async fn terra_get_coin_balance(coin: &str, address: &str) -> Result<Decimal, Box<dyn Error>> {
    let client = Terra::lcd_client_no_tx("https://fcd.terra.dev", "columbus-5");
    let result = client.bank().balances(address).await?;

    let denom = match coin {
        "UST" => "uusd",
        "LUNA" => "uluna",
        _ => {
            return Err(Box::new(IoError::new(
                ErrorKind::Other,
                "invalid coin".to_string(),
            )))
        }
    };

    let mut amount = Decimal::new(1, 6);

    for x in result.result.iter() {
        if x.denom == denom {
            amount *= x.amount;
            break;
        }
    }
    Ok(amount)
}

async fn terra_get_cw20_token_balance(
    token_address: &str,
    address: &str,
) -> Result<Decimal, Box<dyn Error>> {
    let client = Terra::lcd_client_no_tx("https://fcd.terra.dev", "columbus-5");
    let query = "%7B%22token_info%22%3A%7B%7D%7D";

    let value: Value = client.wasm().query(token_address, query).await?;
    if let Value::Null = value["result"]["decimals"] {
        return Err(Box::new(IoError::new(
            ErrorKind::Other,
            "query failed".to_string(),
        )));
    }
    let decimals: u32 = value["result"]["decimals"].to_string().parse()?;

    let query = format!(
        "%7B%22balance%22%3A%7B%22address%22%3A%22{}%22%7D%7D",
        address
    );
    let value: Value = client.wasm().query(token_address, query.as_str()).await?;
    if let Value::Null = value["result"]["balance"] {
        return Err(Box::new(IoError::new(
            ErrorKind::Other,
            "query failed".to_string(),
        )));
    }

    let amount = match &value["result"]["balance"] {
        Value::String(v) => Decimal::from_str_radix(v, 10)?,
        _ => {
            return Err(Box::new(IoError::new(
                ErrorKind::Other,
                "Parse balance result error".to_string(),
            )))
        }
    };

    let decimals = Pow::pow(Decimal::new(10, 0), decimals as u64);
    Ok(amount / decimals)
}

pub async fn get_balance(
    chain_id: &str,
    token: Option<&str>,
    address: &str,
) -> Result<Decimal, Box<dyn Error>> {
    match chain_id {
        "ethereum" | "polygon" | "avalanche" | "moonbeam" | "bsc" => match token {
            Some(token_addr) => web3_get_erc20_token_balance(chain_id, token_addr, address).await,
            None => web3_get_balance(chain_id, address).await,
        },
        "polkadot" => substrate_get_balance(chain_id, address).await,
        "terra" => match token {
            Some(token) => {
                if token.len() > 4 {
                    terra_get_cw20_token_balance(token, address).await
                } else {
                    terra_get_coin_balance(token, address).await
                }
            }
            None => Err(Box::new(IoError::new(
                ErrorKind::Other,
                "no token specified".to_string(),
            ))),
        },
        _ => Err(Box::new(IoError::new(
            ErrorKind::Other,
            "invalid chain_id".to_string(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctor::ctor;

    #[cfg(test)]
    #[ctor]
    fn init() {
        dotenv::from_filename(".env.example").ok();
    }

    #[tokio::test]
    async fn test_web3_get_balance() {
        for chain in ["ethereum", "polygon", "avalanche", "moonbeam", "bsc"].iter() {
            let balance = web3_get_balance(chain, "0xEFb616A5cdE977f87A9878EbEC0b23c655bac762")
                .await
                .unwrap();
            println!("addr balance for {}: {}", chain, balance);
            assert_eq!(Decimal::new(0, 0), balance);
        }
    }

    #[tokio::test]
    async fn test_web3_get_erc20_token_balance() {
        let balance = web3_get_erc20_token_balance(
            "ethereum",
            "0xab16e0d25c06cb376259cc18c1de4aca57605589",
            "0xefb616a5cde977f87a9878ebec0b23c655bac762",
        )
        .await
        .unwrap();
        println!("ERC20: {}", balance);
        assert_eq!(Decimal::new(100, 0), balance);
    }

    #[tokio::test]
    async fn test_polkadot_get_balance() {
        let balance = polkadot_get_balance("16ij7XU6wqSQU5ELKPxmrDotQM6gdwRCE5TeSRDe5D1vKXPY")
            .await
            .unwrap();
        println!("DOT: {}", balance);
        assert_ne!(Decimal::new(0, 0), balance);
    }

    #[tokio::test]
    async fn test_terra_get_coin_balance() {
        let balance =
            terra_get_coin_balance("LUNA", "terra107q76k5uu3atgwz695vdcfee5qz9ukyz3jj0cs")
                .await
                .unwrap();
        println!("LUNA: {}", balance);
        assert_ne!(Decimal::new(0, 0), balance);
    }

    #[tokio::test]
    async fn test_terra_get_cw20_token_balance() {
        let balance = terra_get_cw20_token_balance(
            "terra1hzh9vpxhsk8253se0vv5jj6etdvxu3nv8z07zu",
            "terra107q76k5uu3atgwz695vdcfee5qz9ukyz3jj0cs",
        )
        .await
        .unwrap();
        println!("aUST: {}", balance);
        assert_ne!(Decimal::new(0, 0), balance);
    }
}
