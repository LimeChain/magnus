use std::{collections::HashMap, fs, str::FromStr, sync::Mutex};

use eyre::eyre;
use magnus_shared::Dex;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    AccountMap, Markets, StateAccountToMarket,
    adapters::amms::{
        Amm,
        humidifi::{Humidifi, HumidifiCfg},
    },
};

pub fn load(file: &str) -> eyre::Result<Vec<Box<dyn Amm>>> {
    let json = fs::read_to_string(file)?;

    let cfgs: serde_json::Value = serde_json::from_str(&json)?;

    let pmms = if let serde_json::Value::Array(items) = &cfgs {
        items
            .iter()
            .map(|item| -> Box<dyn Amm> {
                let dex = Dex::from_str(item.get("dex").and_then(|dex| dex.as_str()).expect("no DEX provided")).map_err(|e| eyre!(e)).expect("");

                let init: Box<dyn Amm> = match dex {
                    Dex::Humidifi => {
                        let cfg = HumidifiCfg::try_from(item).map_err(|e| eyre!(e)).expect("");
                        let amm = Humidifi::new(cfg);

                        Box::new(amm)
                    }
                    _ => panic!("Unsupported DEX: {}", dex),
                };

                init
            })
            .collect()
    } else {
        vec![]
    };

    Ok(pmms)
}

/// Creates a mapping from each account address to its parent market key.
/// This is used to route account updates to the correct AMM.
pub fn map_accs_to_market(pmms: &[Box<dyn Amm>]) -> StateAccountToMarket {
    pmms.iter()
        .flat_map(|pmm| {
            let key = pmm.key();
            pmm.get_accounts_to_update().into_iter().map(move |acc| (acc, key))
        })
        .collect()
}

pub fn into_markets(pmms: Vec<Box<dyn Amm>>) -> Markets {
    let map: HashMap<Pubkey, Box<dyn Amm>> = pmms.into_iter().map(|amm| (amm.key(), amm)).collect();

    std::sync::Arc::new(Mutex::new(map))
}

/// Fetches account data for all tracked accounts from the RPC client.
pub async fn acquire_account_map(client: &RpcClient, markets: &Markets) -> eyre::Result<AccountMap> {
    let market_keys: Vec<Pubkey> = markets.lock().unwrap().keys().cloned().collect();
    let accs = client.get_multiple_accounts(&market_keys).await?;

    let acc_map = market_keys.into_iter().zip(accs).filter_map(|(key, acc_opt)| acc_opt.map(|acc| (key, acc))).collect();

    Ok(acc_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_accs_to_market_empty() {
        let pmms: Vec<Box<dyn Amm>> = vec![];
        let map = map_accs_to_market(&pmms);
        assert!(map.is_empty());
    }

    #[test]
    fn test_into_markets_empty() {
        let pmms: Vec<Box<dyn Amm>> = vec![];
        let markets = into_markets(pmms);
        assert!(markets.lock().unwrap().is_empty());
    }
}
