use std::{collections::HashMap, str::FromStr};

use solana_sdk::pubkey::Pubkey;

#[derive(Copy, Clone, Debug, Default)]
pub struct Bootstrap;

impl Bootstrap {
    pub async fn ingest_from_jupiter() -> eyre::Result<Vec<Market>> {
        let response = reqwest::get("https://cache.jup.ag/markets?v=4").await?;
        let markets: Vec<BootstrapMarketData> = response.json().await?;
        markets.into_iter().map(Market::try_from).collect()
    }

    pub fn ingest_from_file(file: &str) -> eyre::Result<Vec<Market>> {
        let content = std::fs::read_to_string(file)?;
        let markets: Vec<BootstrapMarketData> = serde_json::from_str(&content)?;
        markets.into_iter().map(Market::try_from).collect()
    }

    pub fn transform_market_to_owner(markets: &Vec<Market>) -> HashMap<Pubkey, Market> {
        markets.iter().map(|market| (market.owner, market.clone())).collect()
    }

    pub fn transform_market_to_dex(markets: &Vec<Market>) -> HashMap<Pubkey, Market> {
        markets.iter().map(|market| (market.pubkey, market.clone())).collect()
    }

    pub fn transform_market_to_lookup_table(markets: &Vec<Market>) -> HashMap<Pubkey, Market> {
        markets.iter().map(|market| (market.lookup_table, market.clone())).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Market {
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub lookup_table: Pubkey,
    pub routing_group: u8,
    //pub swap_size: SwapAccountSize,
}

impl TryFrom<BootstrapMarketData> for Market {
    type Error = eyre::Error;

    fn try_from(data: BootstrapMarketData) -> Result<Self, Self::Error> {
        Ok(Self {
            pubkey: Pubkey::from_str(&data.pubkey)?,
            owner: Pubkey::from_str(&data.owner)?,
            lookup_table: Pubkey::from_str(&data.params.address_lookup_table_address)?,
            routing_group: data.params.routing_group,
            //swap_size: data.params.swap_account_size,
        })
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct BootstrapMarketData {
    pubkey: String,
    owner: String,
    params: BootstrapParams,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapParams {
    address_lookup_table_address: String,
    routing_group: u8,
    //swap_account_size: SwapAccountSize,
}

//#[derive(Copy, Clone, Debug, serde::Deserialize)]
//pub struct SwapAccountSize {
//    account_compressed_count: u8,
//    account_len: u8,
//    account_metas_count: u8,
//}
