use std::fmt;

use anyhow::Context;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use solana_instruction::Instruction;
use solana_sdk::{
    instruction::AccountMeta,
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    sysvar,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};

/*
 * $ solana --url=mainnet-beta program dump SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe solfi.so
 * $ ..
 */

const TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_PROGRAM_2022: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const MEMO_PROGRAM: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

// v1 btw.
const SOLFI_PROGRAM_ID: Pubkey = pubkey!("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
const SOLFI_WBTC_USDC_MARKET: Pubkey = pubkey!("6LDKXn2hqEtdW1r9jH2ykv5j4y3n4EPt1ZHDn5iVZgck");

const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const WBTC_MINT: Pubkey = pubkey!("5XZw2LKTyrfvfiskJ78AMpackRjPcyCif1WhUsPDuVqQ");

async fn fund_token_account(rpc_client: &Client, rpc_url: &str, owner: &Pubkey, token_account: Pubkey, mint: &Pubkey, amount: u64) -> anyhow::Result<()> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "surfnet_setTokenAccount",
        "params": [
            owner.to_string(),
            mint.to_string(),
            json!({
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

    info!("Funded {} tokens to {}'s ATA: {}", amount, mint, token_account);
    Ok(())
}

async fn fund_sol(rpc_client: &Client, rpc_url: &str, account: &Pubkey, amount: u64) -> anyhow::Result<()> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "surfnet_setAccount",
        "params": [
            account.to_string(),
            json!({
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

    info!("Funded {} SOL to {}", amount, account);
    Ok(())
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_target(true)
        .with_timer(UtcTime::rfc_3339())
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::default().add_directive(tracing::Level::INFO.into())))
        .init();

    let SOL_HTTP_URL = std::env::var("SOL_HTTP_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let SOL_WS_URL = std::env::var("SOL_WS_URL").unwrap_or("ws://127.0.0.1:8900".to_string());

    let client_http = solana_client::rpc_client::RpcClient::new(&SOL_HTTP_URL);
    let client_ws = solana_client::nonblocking::pubsub_client::PubsubClient::new(&SOL_WS_URL).await.expect("unable to create websocket client");
    let client_req = reqwest::Client::new();

    let acc1 = Keypair::new();
    let acc1_token_usdc = spl_associated_token_account::get_associated_token_address(&acc1.pubkey(), &USDC_MINT);
    let acc1_token_wbtc = spl_associated_token_account::get_associated_token_address(&acc1.pubkey(), &WBTC_MINT);

    fund_sol(&client_req, &SOL_HTTP_URL, &acc1.pubkey(), 1_000_000_000).await?;
    fund_token_account(&client_req, &SOL_HTTP_URL, &acc1.pubkey(), acc1_token_usdc, &USDC_MINT, 1_000_000_000_000).await?;
    fund_token_account(&client_req, &SOL_HTTP_URL, &acc1.pubkey(), acc1_token_wbtc, &WBTC_MINT, 1_000_000_000_000).await?;

    info!(
        "\n[after initial funding] acc1 ({}), Balance [SOL]: {}, token_acc1 ({}) Balance [USDC]: {}, Balance [WBTC]: {}",
        acc1.pubkey(),
        client_http.get_balance(&acc1.pubkey())?,
        acc1_token_usdc,
        client_http.get_token_account_balance(&acc1_token_usdc).map(|b| b.ui_amount.unwrap_or(0.0))?,
        client_http.get_token_account_balance(&acc1_token_wbtc).map(|b| b.ui_amount.unwrap_or(0.0))?,
    );

    let mut ins = vec![];

    ins.push(create_swap_ix(SwapDirection::WbtcToUsdc, &SOLFI_WBTC_USDC_MARKET, &acc1.pubkey(), &USDC_MINT, &WBTC_MINT, 1_u64.pow(6)));

    //let blockhash = client_http.get_latest_blockhash()?;
    //let tx = Transaction::new_with_payer(&ins, Some(&acc1.pubkey()));
    //let signed_tx = Transaction::new(&[acc1], tx.message, blockhash);

    //client_http.send_transaction(&signed_tx)?;

    let blockhash = client_http.get_latest_blockhash()?;
    let mut tx = Transaction::new_with_payer(&ins, Some(&acc1.pubkey()));
    tx.sign(&[&acc1], blockhash);
    client_http.send_transaction(&tx)?;

    Ok(())
}

const DISCRIMINATOR: u8 = 7;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize)]
pub enum SwapDirection {
    #[default]
    WbtcToUsdc,
    UsdcToWbtc,
}

impl fmt::Display for SwapDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapDirection::WbtcToUsdc => write!(f, "wbtc-to-usdc"),
            SwapDirection::UsdcToWbtc => write!(f, "usdc-to-wbtc"),
        }
    }
}

fn create_instruction_data(direction: SwapDirection, amount_in: u64) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(9);
    buffer.push(DISCRIMINATOR);
    buffer.extend_from_slice(&amount_in.to_le_bytes());
    buffer.resize(18, 0);
    buffer[17] = direction as u8;
    buffer
}

pub fn create_swap_ix(direction: SwapDirection, market: &Pubkey, user: &Pubkey, token_a: &Pubkey, token_b: &Pubkey, amount: u64) -> Instruction {
    Instruction {
        program_id: SOLFI_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*market, false),
            AccountMeta::new(get_associated_token_address(market, token_a), false),
            AccountMeta::new(get_associated_token_address(market, token_b), false),
            AccountMeta::new(get_associated_token_address(user, token_a), false),
            AccountMeta::new(get_associated_token_address(user, token_b), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ],
        data: create_instruction_data(direction, amount),
    }
}
