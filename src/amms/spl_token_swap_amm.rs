use std::{collections::HashMap, convert::TryInto, sync::LazyLock};

use anchor_lang::{prelude::Pubkey, pubkey};
//use anchor_lang::prelude::Pubkey;
use anyhow::{Context, Result, ensure};
use jupiter_amm_interface::{AccountMap, AmmContext, KeyedAccount, Quote, QuoteParams, SwapAndAccountMetas, SwapParams, try_get_account_data};
use solana_program::program_pack::Pack;
//use solana_sdk::program_pack::Pack;
//use program_interfaces::jupiter_dex_interfaces::client::accounts::TokenSwap;
use spl_token::state::Account as TokenAccount;

use crate::{
    amm::{Amm, Swap, to_dex_account_metas},
    amms::{account_meta_from_token_swap::TokenSwap, *},
    curves::{
        base::{CurveType, SwapCurve},
        calculator::TradeDirection,
    },
    math::swap_curve_info::get_swap_curve_result,
    state::SwapV1,
};

pub struct ConstantProductAmm {
    key: Pubkey,
    authority: Pubkey,
    label: String,
    state: SwapV1,
    reserve_mints: [Pubkey; 2],
    reserves: [u128; 2],
    program_id: Pubkey,
}

impl Clone for ConstantProductAmm {
    fn clone(&self) -> Self {
        ConstantProductAmm {
            key: self.key,
            authority: self.authority,
            label: self.label.clone(),
            state: SwapV1 {
                is_initialized: self.state.is_initialized,
                bump_seed: self.state.bump_seed,
                token_program_id: self.state.token_program_id,
                token_a: self.state.token_a,
                token_b: self.state.token_b,
                pool_mint: self.state.pool_mint,
                token_a_mint: self.state.token_a_mint,
                token_b_mint: self.state.token_b_mint,
                pool_fee_account: self.state.pool_fee_account,
                fees: self.state.fees.clone(),
                swap_curve: SwapCurve { curve_type: self.state.swap_curve.curve_type, calculator: self.state.swap_curve.calculator.clone() },
            },
            reserve_mints: self.reserve_mints,
            program_id: self.program_id,
            reserves: self.reserves,
        }
    }
}

impl Amm for ConstantProductAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        // Skip the first byte which is version
        let state = SwapV1::unpack(&keyed_account.account.data[1..])?;

        // Support only the most common non exotic curves
        ensure!(matches!(state.swap_curve.curve_type, CurveType::ConstantProduct | CurveType::Stable));

        let reserve_mints = [state.token_a_mint, state.token_b_mint];

        // export outside on a per-exchange basis
        let label = "..".to_string();

        let program_id = keyed_account.account.owner;
        Ok(Self {
            key: keyed_account.key,
            authority: Pubkey::find_program_address(&[&keyed_account.key.to_bytes()], &program_id).0,
            label,
            state,
            reserve_mints,
            program_id,
            reserves: Default::default(),
        })
    }

    fn label(&self) -> String {
        self.label.clone()
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.reserve_mints.to_vec()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.state.token_a, self.state.token_b]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let token_a_account = try_get_account_data(account_map, &self.state.token_a)?;
        let token_a_token_account = TokenAccount::unpack(token_a_account)?;

        let token_b_account = try_get_account_data(account_map, &self.state.token_b)?;
        let token_b_token_account = TokenAccount::unpack(token_b_account)?;

        self.reserves = [token_a_token_account.amount.into(), token_b_token_account.amount.into()];

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        let (trade_direction, swap_source_amount, swap_destination_amount) = if quote_params.input_mint == self.reserve_mints[0] {
            (TradeDirection::AtoB, self.reserves[0], self.reserves[1])
        } else {
            (TradeDirection::BtoA, self.reserves[1], self.reserves[0])
        };

        let swap_result = get_swap_curve_result(&self.state.swap_curve, quote_params.amount, swap_source_amount, swap_destination_amount, trade_direction, &self.state.fees)?;

        Ok(Quote {
            fee_pct: swap_result.fee_pct,
            in_amount: swap_result.input_amount.try_into()?,
            out_amount: swap_result.expected_output_amount.try_into()?,
            fee_amount: swap_result.fees.try_into()?,
            fee_mint: quote_params.input_mint,
        })
    }

    fn get_accounts_len(&self) -> usize {
        11
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let SwapParams { source_mint, destination_token_account, source_token_account, token_transfer_authority, .. } = swap_params;

        let (swap_source, swap_destination) = if *source_mint == self.state.token_a_mint {
            (self.state.token_a, self.state.token_b)
        } else {
            (self.state.token_b, self.state.token_a)
        };

        Ok(SwapAndAccountMetas {
            swap: Swap::TokenSwap,
            account_metas: to_dex_account_metas(
                self.program_id,
                TokenSwap {
                    token_swap_program: self.program_id,
                    token_program: spl_token::ID,
                    swap: self.key,
                    authority: self.authority,
                    user_transfer_authority: *token_transfer_authority,
                    source: *source_token_account,
                    swap_source,
                    swap_destination,
                    destination: *destination_token_account,
                    pool_mint: self.state.pool_mint,
                    pool_fee: self.state.pool_fee_account,
                },
            ),
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }
}
