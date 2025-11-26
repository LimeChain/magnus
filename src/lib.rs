//! Magnus is a modular Solana solver; There are a few things to note here:
//! TODO
//! |1| - ..
//! |2| - ..
//! |3| - ..
pub mod adapters;
pub mod bootstrap;
pub mod curves;
pub mod error;
pub mod geyser_client;
pub mod helpers;
pub mod ingest;
pub mod propagate;
// pub mod solve;

// tmp
use anyhow::Context;

pub async fn fund_token_account(
    rpc_client: &reqwest::Client,
    rpc_url: &str,
    owner: &solana_sdk::pubkey::Pubkey,
    token_account: solana_sdk::pubkey::Pubkey,
    mint: &solana_sdk::pubkey::Pubkey,
    amount: u64,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "surfnet_setTokenAccount",
        "params": [
            owner.to_string(),
            mint.to_string(),
            serde_json::json!({
                "amount": amount,
                "state": "initialized"
            })
        ]
    });

    let res = rpc_client.post(rpc_url).json(&payload).send().await.context("Failed to send surfnet_setTokenAccount RPC request")?;
    let res_body: serde_json::Value = res.json().await?;
    if res_body["error"].is_object() {
        return Err(anyhow::anyhow!("Surfpool token funding error: {:?}", res_body["error"]));
    }

    tracing::info!("Funded {} tokens to {}'s ATA: {}", amount, mint, token_account);
    Ok(())
}

pub async fn fund_sol(rpc_client: &reqwest::Client, rpc_url: &str, account: &solana_sdk::pubkey::Pubkey, amount: u64) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "surfnet_setAccount",
        "params": [
            account.to_string(),
            serde_json::json!({
                "lamports": amount,
                "owner": "11111111111111111111111111111111"
            })
        ]
    });

    let res = rpc_client.post(rpc_url).json(&payload).send().await.context("Failed to send surfnet_setTokenAccount RPC request")?;
    let res_body: serde_json::Value = res.json().await?;
    if res_body["error"].is_object() {
        return Err(anyhow::anyhow!("Surfpool SOL funding error: {:?}", res_body["error"]));
    }

    tracing::info!("Funded {} SOL to {}", amount, account);
    Ok(())
}
