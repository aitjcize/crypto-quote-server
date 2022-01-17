use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::{Cursor, Error as IoError, ErrorKind};

use ethcontract::contract::ViewMethodBuilder;
use ethcontract::Instance;
use ethcontract_common::{hash, Abi};
use reqwest;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;
use web3::types::{Address, U256};
use web3::Web3;

const ERC20_ABI: &str = r#"[{"inputs":[{"internalType":"string","name":"name_","type":"string"},{"internalType":"string","name":"symbol_","type":"string"}],"stateMutability":"nonpayable","type":"constructor"},{"anonymous":false,"inputs":[{"indexed":true,"internalType":"address","name":"owner","type":"address"},{"indexed":true,"internalType":"address","name":"spender","type":"address"},{"indexed":false,"internalType":"uint256","name":"value","type":"uint256"}],"name":"Approval","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"internalType":"address","name":"from","type":"address"},{"indexed":true,"internalType":"address","name":"to","type":"address"},{"indexed":false,"internalType":"uint256","name":"value","type":"uint256"}],"name":"Transfer","type":"event"},{"inputs":[],"name":"name","outputs":[{"internalType":"string","name":"","type":"string"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"symbol","outputs":[{"internalType":"string","name":"","type":"string"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"decimals","outputs":[{"internalType":"uint8","name":"","type":"uint8"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"totalSupply","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"account","type":"address"}],"name":"balanceOf","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"recipient","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"address","name":"owner","type":"address"},{"internalType":"address","name":"spender","type":"address"}],"name":"allowance","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"spender","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"approve","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"address","name":"sender","type":"address"},{"internalType":"address","name":"recipient","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"transferFrom","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"address","name":"spender","type":"address"},{"internalType":"uint256","name":"addedValue","type":"uint256"}],"name":"increaseAllowance","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"address","name":"spender","type":"address"},{"internalType":"uint256","name":"subtractedValue","type":"uint256"}],"name":"decreaseAllowance","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"}]"#;

fn get_web3_transport(chain_id: &str) -> Result<Web3<web3::transports::Http>, Box<dyn Error>> {
    let rpc_endpoint = match chain_id {
        "ethereum" => {
            let token = env::var("INFURA_TOKEN").unwrap();
            Ok(format!("https://mainnet.infura.io/v3/{}", token))
        }
        "polygon" => Ok("https://polygon-rpc.com".to_string()),
        "avalanche" => Ok("https://api.avax.network/ext/bc/C/rpc".to_string()),
        _ => Err(IoError::new(
            ErrorKind::Other,
            format!("invalid chain_id id"),
        )),
    }?;
    let transport = web3::transports::Http::new(rpc_endpoint.as_str())?;
    Ok(web3::Web3::new(transport))
}

async fn web3_get_balance(chain_id: &str, address: &str) -> Result<Decimal, Box<dyn Error>> {
    let web3 = get_web3_transport(chain_id)?;
    let balance: Decimal = web3
        .eth()
        .balance(address.parse().unwrap(), None)
        .await
        .unwrap()
        .to_string()
        .parse()
        .unwrap();
    let decimals: Decimal = U256::exp10(18).to_string().parse().unwrap();
    Ok(balance / decimals)
}

async fn web3_get_erc20_token_balance(
    chain_id: &str,
    contract_address: &str,
    address: &str,
) -> Result<Decimal, Box<dyn Error>> {
    let web3 = get_web3_transport(chain_id)?;
    let f = Cursor::new(ERC20_ABI.as_bytes().to_vec());
    let instance = Instance::at(web3, Abi::load(f).unwrap(), contract_address.parse()?);
    let address: Address = address.parse()?;
    let v: ViewMethodBuilder<_, u8> =
        instance.view_method(hash::function_selector("decimals()"), ())?;
    let decimals: u8 = v.call().await?;
    let decimals: Decimal = U256::exp10(decimals as usize).to_string().parse().unwrap();

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
    struct Result {
        data: Data,
    }

    let r: Result = resp.json::<Result>().await?;
    Ok(r.data.balance)
}

async fn substrate_get_balance(chain_id: &str, address: &str) -> Result<Decimal, Box<dyn Error>> {
    match chain_id {
        "polkadot" => polkadot_get_balance(address).await,
        _ => Err(Box::new(IoError::new(
            ErrorKind::Other,
            format!("invalid chain_id"),
        ))),
    }
}

pub async fn get_balance(
    chain_id: &str,
    token_address: Option<&str>,
    address: &str,
) -> Result<Decimal, Box<dyn Error>> {
    match chain_id {
        "ethereum" | "polygon" | "avalanche" => match token_address {
            Some(token_addr) => web3_get_erc20_token_balance(chain_id, token_addr, address).await,
            None => web3_get_balance(chain_id, address).await,
        },
        "polkadot" => substrate_get_balance(chain_id, address).await,
        _ => Err(Box::new(IoError::new(
            ErrorKind::Other,
            format!("invalid chain_id"),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctor::ctor;
    use dotenv::dotenv;

    #[cfg(test)]
    #[ctor]
    fn init() {
        dotenv().ok();
    }

    #[tokio::test]
    async fn test_web3_get_balance() {
        let balance = web3_get_balance("ethereum", "0xEFb616A5cdE977f87A9878EbEC0b23c655bac762")
            .await
            .unwrap();
        assert_eq!(Decimal::new(0, 0), balance);
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
        assert_eq!(Decimal::new(100, 0), balance);
    }

    #[tokio::test]
    async fn test_polkadot_get_balance() {
        let balance = polkadot_get_balance("16ij7XU6wqSQU5ELKPxmrDotQM6gdwRCE5TeSRDe5D1vKXPY")
            .await
            .unwrap();
        assert_ne!(Decimal::new(0, 0), balance);
    }
}
