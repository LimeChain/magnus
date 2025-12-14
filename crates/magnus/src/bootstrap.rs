use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use ahash::HashMapExt;
use magnus_consts::amm_raydium_cp;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    AccountMap, Markets, Programs,
    adapters::amms::{Amm, obric_v2::ObricV2, raydium_cp::RaydiumCP},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Bootstrap;

impl Bootstrap {
    pub async fn ingest_from_jupiter() -> eyre::Result<Vec<MarketRaw>> {
        let response = reqwest::get("https://cache.jup.ag/markets?v=4").await?;
        let markets: Vec<BootstrapMarketData> = response.json().await?;
        markets.into_iter().map(MarketRaw::try_from).collect()
    }

    pub fn ingest_from_file(file: &str) -> eyre::Result<Vec<MarketRaw>> {
        let content = std::fs::read_to_string(file)?;
        let markets: Vec<BootstrapMarketData> = serde_json::from_str(&content)?;
        markets.into_iter().map(MarketRaw::try_from).collect()
    }

    /// acquires all the programs for whom we're following one or more markets.
    pub fn transform_market_to_owner(markets: &Vec<MarketRaw>) -> Vec<Pubkey> {
        markets.iter().map(|market| market.owner).collect()
    }

    pub fn transform_market_to_dex(markets: &Vec<MarketRaw>) -> HashMap<Pubkey, MarketRaw> {
        markets.iter().map(|market| (market.pubkey, market.clone())).collect()
    }

    pub fn get_program_markets(markets: &Vec<MarketRaw>) -> Programs {
        let mut program_markets = Programs::new();

        markets.iter().for_each(|market| {
            if let Some(m) = program_markets.get_mut(&market.owner) {
                m.push(market.pubkey);
            } else {
                program_markets.insert(market.owner, vec![market.pubkey]);
            }
        });

        program_markets
    }

    /// Initialises the corresponding markets based on the provided programs
    pub async fn init_markets(program_markets: Programs) -> eyre::Result<Markets> {
        let obric_v2_id = ObricV2::default().program_id();
        let raydium_cp_id = RaydiumCP::default().program_id();

        let ir: HashMap<Pubkey, Box<dyn Amm>> = program_markets
            .iter()
            .flat_map(|(program, markets)| {
                markets.iter().map(move |market| {
                    let amm: Box<dyn Amm> = match program {
                        p if *p == obric_v2_id => Box::new(ObricV2::new()),
                        p if *p == raydium_cp_id => Box::new(RaydiumCP::new()),
                        _ => unimplemented!("..."),
                    };
                    (*market, amm)
                })
            })
            .collect();

        Ok(Arc::new(Mutex::new(ir)))
    }

    /// https://www.helius.dev/docs/rpc/guides/getmultipleaccounts#response-structure
    /// Each account in the vec responds to the same index of the markets_addrs vec.
    pub async fn acquire_account_map(client: &RpcClient, markets: &Markets) -> eyre::Result<AccountMap> {
        let markets_addrs: Vec<Pubkey> = markets.lock().unwrap().keys().cloned().collect();
        let accs = client.get_multiple_accounts(&markets_addrs).await?;
        let mut acc_map = AccountMap::new();
        let mut counter = 0;

        accs.iter().for_each(|am| {
            if let Some(account) = am {
                acc_map.insert(markets_addrs[counter], account.clone());
            }

            counter += 1;
        });

        Ok(acc_map)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MarketRaw {
    pub pubkey: Pubkey,
    pub owner: Pubkey,
}

impl TryFrom<BootstrapMarketData> for MarketRaw {
    type Error = eyre::Error;

    fn try_from(data: BootstrapMarketData) -> Result<Self, Self::Error> {
        Ok(Self { pubkey: Pubkey::from_str(&data.pubkey)?, owner: Pubkey::from_str(&data.owner)? })
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct BootstrapMarketData {
    pubkey: String,
    owner: String,
}

#[cfg(test)]
mod tests {
    /* .. */
}
