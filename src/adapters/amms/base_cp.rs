/*
 * CP-AMM adapters rely on the same principles:
 * - x*y = k
 * - two reserves representing the assets in the pool
 */

use std::convert::TryInto;

use solana_program::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;

pub fn try_get_account_data<'a>(account_map: &'a AccountMap, address: &Pubkey) -> eyre::Result<&'a [u8]> {
    account_map.get(address).map(|account| account.data.as_slice()).ok_or_else(|| eyre::eyre!("Could not find address: {address}"))
}

use crate::{
    adapters::{
        Adapter, Quote, QuoteParams, Swap, SwapAndAccountMetas, SwapParams,
        amms::{AccountMap, Amm, AmmContext, KeyedAccount, swap_state::SwapV1},
        helpers::{TokenSwap, get_swap_curve_result, to_dex_account_metas},
    },
    curves::{base::SwapCurve, calculator::TradeDirection},
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

impl Adapter for ConstantProductAmm {}

impl Amm for ConstantProductAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> eyre::Result<Self> {
        let state = SwapV1::unpack(&keyed_account.account.data[1..])?;
        let reserve_mints = [state.token_a_mint, state.token_b_mint];

        // TODO: export outside on a per-exchange basis
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

    fn update(&mut self, account_map: &AccountMap) -> eyre::Result<()> {
        let token_a_account = try_get_account_data(account_map, &self.state.token_a)?;
        let token_a_token_account = TokenAccount::unpack(token_a_account)?;

        let token_b_account = try_get_account_data(account_map, &self.state.token_b)?;
        let token_b_token_account = TokenAccount::unpack(token_b_account)?;

        self.reserves = [token_a_token_account.amount.into(), token_b_token_account.amount.into()];

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> eyre::Result<Quote> {
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

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> eyre::Result<SwapAndAccountMetas> {
        let SwapParams { source_mint, destination_token_account, source_token_account, token_transfer_authority, .. } = swap_params;

        let (swap_source, swap_destination) = if *source_mint == self.state.token_a_mint {
            (self.state.token_a, self.state.token_b)
        } else {
            (self.state.token_b, self.state.token_a)
        };

        Ok(SwapAndAccountMetas {
            swap: Swap::Base,
            account_metas: to_dex_account_metas(
                self.program_id,
                TokenSwap {
                    token_swap_program: self.program_id,
                    token_program: Pubkey::new_from_array(spl_token::ID.to_bytes()),
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

    fn clone_amm(&self) -> Box<dyn Amm> {
        Box::new(self.clone())
    }
}
