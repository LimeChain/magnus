//! https://docs.surfpool.run/rpc/cheatcodes

use eyre::Context;
use reqwest::Client;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;

pub struct SurfpoolClient {
    http_client: Client,
    rpc_url: String,
}

impl SurfpoolClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self { http_client: Client::new(), rpc_url: rpc_url.into() }
    }

    pub fn with_client(http_client: Client, rpc_url: impl Into<String>) -> Self {
        Self { http_client, rpc_url: rpc_url.into() }
    }

    pub async fn fund_token_account(&self, owner: &Pubkey, token_account: &Pubkey, mint: &Pubkey, amount: u64) -> eyre::Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_setTokenAccount",
            "params": [
                owner.to_string(),
                mint.to_string(),
                {
                    "amount": amount,
                    "state": "initialized"
                }
            ]
        });

        let res = self.http_client.post(&self.rpc_url).json(&payload).send().await.context("Failed to send surfnet_setTokenAccount RPC request")?;
        let res_body: serde_json::Value = res.json().await?;

        if res_body["error"].is_object() {
            return Err(eyre::eyre!("Surfpool token funding error: {:?}", res_body["error"]));
        }

        tracing::info!("Funded {} tokens to {}'s ATA: {}", amount, mint, token_account);
        Ok(())
    }

    pub async fn fund_sol(&self, account: &Pubkey, amount: u64) -> eyre::Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_setAccount",
            "params": [
                account.to_string(),
                {
                    "lamports": amount,
                    "owner": "11111111111111111111111111111111"
                }
            ]
        });

        let res = self.http_client.post(&self.rpc_url).json(&payload).send().await.context("Failed to send surfnet_setAccount RPC request")?;
        let res_body: serde_json::Value = res.json().await?;

        if res_body["error"].is_object() {
            return Err(eyre::eyre!("Surfpool SOL funding error: {:?}", res_body["error"]));
        }

        tracing::info!("Funded {} SOL to {}", amount, account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SurfpoolClient::new("http://localhost:8899");
        assert_eq!(client.rpc_url, "http://localhost:8899");
    }
}
