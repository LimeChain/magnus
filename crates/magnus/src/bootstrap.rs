use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use ahash::HashMapExt;
use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    AccountMap, Markets, Programs,
    adapters::amms::{Amm, humidifi::Humidifi, obric_v2::ObricV2, raydium_cl_v2::RaydiumCLV2, raydium_cp::RaydiumCP},
};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Bootstrap {
    pub markets_raw: Vec<MarketRaw>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct MarketRaw {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub pubkey: Pubkey,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub owner: Pubkey,
    // optional list of accounts to follow for updates
    // used by amms with unknown IDLs — i.e markets whose state we cannot deserialise into
    #[serde(default)]
    pub accounts: Option<Vec<String>>,
}

fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

impl Bootstrap {
    pub async fn ingest_from_jupiter() -> eyre::Result<Bootstrap> {
        let response = reqwest::get("https://cache.jup.ag/markets?v=4").await?;
        let markets: Vec<MarketRaw> = response.json().await?;

        Ok(Bootstrap { markets_raw: markets })
    }

    pub fn ingest_from_file(file: &str) -> eyre::Result<Bootstrap> {
        let content = std::fs::read_to_string(file)?;
        let markets: Vec<MarketRaw> = serde_json::from_str(&content)?;

        Ok(Bootstrap { markets_raw: markets })
    }

    /// acquires all the programs for whom we're following one or more markets.
    pub fn transform_market_to_owner(markets: &[MarketRaw]) -> Vec<Pubkey> {
        markets.iter().map(|market| market.owner).collect()
    }

    pub fn transform_markets(markets: &[MarketRaw]) -> HashMap<Pubkey, MarketRaw> {
        markets.iter().map(|market| (market.pubkey, market.clone())).collect()
    }

    pub fn get_program_markets(&self) -> Programs {
        let mut program_markets = Programs::new();

        self.markets_raw.iter().for_each(|market| {
            if let Some(m) = program_markets.get_mut(&market.owner) {
                m.push(market.pubkey);
            } else {
                program_markets.insert(market.owner, vec![market.pubkey]);
            }
        });

        program_markets
    }

    /// Initialises the corresponding markets based on the provided programs
    pub async fn init_markets(program_markets: Programs, markets_raw: &[MarketRaw]) -> eyre::Result<Markets> {
        let raydium_cp_id = RaydiumCP::default().program_id();
        let raydium_cl_v2_id = RaydiumCLV2.program_id();
        let obric_v2_id = ObricV2::default().program_id();
        let humidifi_id = Humidifi::default().program_id();

        let ir: HashMap<Pubkey, Box<dyn Amm>> = program_markets
            .iter()
            .flat_map(|(program, markets)| {
                markets.iter().map(move |market| {
                    let m = match markets_raw.iter().find(|tmp| &tmp.pubkey == market) {
                        Some(m) => m,
                        None => &MarketRaw::default(),
                    };

                    let amm: Box<dyn Amm> = match program {
                        p if *p == raydium_cp_id => Box::new(RaydiumCP::new()),
                        p if *p == raydium_cl_v2_id => Box::new(RaydiumCLV2::new()),
                        p if *p == obric_v2_id => Box::new(ObricV2::new()),
                        p if *p == humidifi_id => Box::new(Humidifi::new(m.pubkey, m.accounts.clone().unwrap_or_default())),
                        _ => unimplemented!("..."),
                    };
                    (*market, amm)
                })
            })
            .collect();

        tracing::info!("??: {:?}", ir);
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

#[cfg(test)]
mod tests {
    use super::*;

    // jupiter returns the markets with a few additional fields
    // like params, but we're not utilising them in any way
    #[test]
    fn test_deserialize_market_raw_with_params() {
        let json = r#"
        {
            "pubkey": "6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck",
            "owner": "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe",
            "params": {
                "addressLookupTableAddress": "BQSBvgQyHfkcoiYtjgHvdARhyZ6JNBKLufEQcaYY4hu",
                "routingGroup": 3,
                "swapAccountSize": {
                    "account_compressed_count": 6,
                    "account_len": 9,
                    "account_metas_count": 9
                }
            }
        }
        "#;

        let market: MarketRaw = serde_json::from_str(json).unwrap();

        assert_eq!(market.pubkey.to_string(), "6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck");
        assert_eq!(market.owner.to_string(), "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
        assert_eq!(market.accounts, None);
    }

    #[test]
    fn test_deserialize_market_raw_without_params() {
        let json = r#"
        {
            "pubkey": "4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT",
            "owner": "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe"
        }
        "#;

        let market: MarketRaw = serde_json::from_str(json).unwrap();

        assert_eq!(market.pubkey.to_string(), "4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT");
        assert_eq!(market.owner.to_string(), "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
        assert_eq!(market.accounts, None);
    }

    #[test]
    fn test_deserialize_bootstrap_full_array() {
        let json = r#"
        [
            {
                "pubkey": "6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck",
                "owner": "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe",
                "params": {
                    "addressLookupTableAddress": "BQSBvgQyHfkcoiYtjgHvdARhyZ6JNBKLufEQcaYY4hu",
                    "routingGroup": 3,
                    "swapAccountSize": {
                        "account_compressed_count": 6,
                        "account_len": 9,
                        "account_metas_count": 9
                    }
                }
            },
            {
                "pubkey": "4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT",
                "owner": "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe"
            }
        ]
        "#;

        let markets: Vec<MarketRaw> = serde_json::from_str(json).unwrap();

        assert_eq!(markets.len(), 2);
        assert_eq!(markets[0].pubkey.to_string(), "6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck");
        assert_eq!(markets[1].pubkey.to_string(), "4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT");
    }

    #[test]
    fn test_transform_market_to_owner() {
        let markets = vec![
            MarketRaw {
                pubkey: Pubkey::from_str("6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck").unwrap(),
                owner: Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap(),
                accounts: None,
            },
            MarketRaw {
                pubkey: Pubkey::from_str("4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT").unwrap(),
                owner: Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap(),
                accounts: None,
            },
        ];

        let owners = Bootstrap::transform_market_to_owner(&markets);

        assert_eq!(owners.len(), 2);
        assert_eq!(owners[0].to_string(), "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
    }

    #[test]
    fn test_transform_market_to_dex() {
        let markets = vec![MarketRaw {
            pubkey: Pubkey::from_str("6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck").unwrap(),
            owner: Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap(),
            accounts: None,
        }];

        let dex_map = Bootstrap::transform_markets(&markets);

        assert_eq!(dex_map.len(), 1);
        let key = Pubkey::from_str("6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck").unwrap();
        assert!(dex_map.contains_key(&key));
    }

    #[test]
    fn test_get_program_markets() {
        let bootstrap = Bootstrap {
            markets_raw: vec![
                MarketRaw {
                    pubkey: Pubkey::from_str("6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck").unwrap(),
                    owner: Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap(),
                    accounts: None,
                },
                MarketRaw {
                    pubkey: Pubkey::from_str("4o9kDwyuBhcCF6mmp78HZHPc5Kdw1AmcSwBpcdyQhZvT").unwrap(),
                    owner: Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap(),
                    accounts: None,
                },
            ],
        };

        let program_markets = bootstrap.get_program_markets();

        assert_eq!(program_markets.len(), 1);
        let owner = Pubkey::from_str("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe").unwrap();
        assert_eq!(program_markets.get(&owner).unwrap().len(), 2);
    }

    #[test]
    fn test_invalid_pubkey() {
        let json = r#"
        {
            "pubkey": "invalid_pubkey",
            "owner": "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe"
        }
        "#;

        let result: Result<MarketRaw, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
