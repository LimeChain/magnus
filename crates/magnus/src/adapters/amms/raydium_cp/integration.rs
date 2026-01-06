use anchor_lang::AccountDeserialize;
use anchor_spl::token::TokenAccount;
use borsh::BorshDeserialize;
use magnus_shared::amm_raydium_cp;
use solana_instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use crate::adapters::{
    Adapter, AmmSwap,
    amms::{AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, SwapAndAccountMetas, SwapParams, raydium_cp},
};

#[derive(Clone, Debug, Default)]
pub struct RaydiumCP {
    key: Pubkey,
    pub state: raydium_cp::state::State,
    current_x: u64,
    current_y: u64,
}

impl RaydiumCP {
    pub fn new() -> RaydiumCP {
        RaydiumCP::default()
    }
}

impl Adapter for RaydiumCP {}

impl Amm for RaydiumCP {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str_const(&amm_raydium_cp::id().to_string())
    }

    fn label(&self) -> String {
        "RaydiumCP".to_string()
    }

    fn get_accounts_len(&self) -> usize {
        13
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![self.state.token_0_mint, self.state.token_1_mint]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.state.token_0_vault, self.state.token_1_vault]
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(RaydiumCP { key: self.key, state: self.state.clone(), current_x: self.current_x, current_y: self.current_y })
    }

    fn from_keyed_account(keyed_account: &KeyedAccount, _: &AmmContext) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        let data = &mut &keyed_account.account.data.clone()[8..];
        let state = raydium_cp::state::State::deserialize(data)?;
        Ok(RaydiumCP { key: keyed_account.key, state, current_x: 0, current_y: 0 })
    }

    fn update(&mut self, account_map: &AccountMap, _: Option<u64>) -> eyre::Result<()> {
        let vault_0 = account_map.get(&self.state.token_0_vault).ok_or_else(|| eyre::eyre!("token_0_vault not found"))?;
        let vault_1 = account_map.get(&self.state.token_1_vault).ok_or_else(|| eyre::eyre!("token_1_vault not found"))?;

        let vault_0_data = TokenAccount::try_deserialize(&mut &vault_0.data[..])?;
        let vault_1_data = TokenAccount::try_deserialize(&mut &vault_1.data[..])?;

        self.current_x = vault_0_data.amount;
        self.current_y = vault_1_data.amount;

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> eyre::Result<Quote> {
        let (input_reserve, output_reserve) = if quote_params.input_mint == self.state.token_0_mint {
            (self.current_x, self.current_y)
        } else {
            (self.current_y, self.current_x)
        };

        if input_reserve == 0 || output_reserve == 0 {
            return Err(eyre::eyre!("insufficient liquidity"));
        }

        let fee_bps = 30u64;
        let amount_in_with_fee = quote_params.amount.checked_mul(10000 - fee_bps).ok_or_else(|| eyre::eyre!("overflow"))?;
        let numerator = amount_in_with_fee.checked_mul(output_reserve).ok_or_else(|| eyre::eyre!("overflow"))?;
        let denominator = input_reserve.checked_mul(10000).ok_or_else(|| eyre::eyre!("overflow"))?.checked_add(amount_in_with_fee).ok_or_else(|| eyre::eyre!("overflow"))?;

        let out_amount = numerator / denominator;
        let fee_amount = quote_params.amount * fee_bps / 10000;

        Ok(Quote { in_amount: quote_params.amount, out_amount, fee_amount, fee_mint: quote_params.input_mint, fee_pct: rust_decimal::Decimal::new(fee_bps as i64, 4) })
    }

    // https://solscan.io/tx/rUwLuvAuE5vKH48c3n7ZUbuUudPqdKsdcBy58gMUopYDg9yC5FbB1feg3xrEuvemBWwCbSjkmAVxqCCLthpBG1h
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> eyre::Result<SwapAndAccountMetas> {
        let (vault_in, vault_out, mint_in, mint_out, program_in, program_out) = if swap_params.input_mint == self.state.token_0_mint {
            (self.state.token_0_vault, self.state.token_1_vault, self.state.token_0_mint, self.state.token_1_mint, self.state.token_0_program, self.state.token_1_program)
        } else {
            (self.state.token_1_vault, self.state.token_0_vault, self.state.token_1_mint, self.state.token_0_mint, self.state.token_1_program, self.state.token_0_program)
        };

        let account_metas = vec![
            AccountMeta::new(swap_params.token_transfer_authority, true),
            AccountMeta::new_readonly(swap_params.token_transfer_authority, false),
            AccountMeta::new_readonly(self.state.amm_config, false),
            AccountMeta::new(self.key(), false),
            AccountMeta::new(swap_params.source_token_account, false),
            AccountMeta::new(swap_params.destination_token_account, false),
            AccountMeta::new(vault_in, false),
            AccountMeta::new(vault_out, false),
            AccountMeta::new_readonly(program_in, false),
            AccountMeta::new_readonly(program_out, false),
            AccountMeta::new_readonly(mint_in, false),
            AccountMeta::new_readonly(mint_out, false),
            AccountMeta::new(self.state.observation_key, false),
        ];

        Ok(SwapAndAccountMetas { swap: AmmSwap::RaydiumCP, account_metas })
    }
}
