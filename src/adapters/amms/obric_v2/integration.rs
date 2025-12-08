use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use borsh::BorshDeserialize;
use eyre::Result;
use solana_instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use crate::adapters::{
    Adapter, Swap,
    amms::{
        AccountMap, Amm, AmmContext, KeyedAccount, OBRIC_V2, Quote, QuoteParams, SPL_TOKEN_PROGRAM_ID, SwapAndAccountMetas, SwapParams,
        obric_v2::state::{PriceFeed, SSTradingPair},
    },
};

#[derive(thiserror::Error, Debug)]
pub enum AmmError {
    #[error("Account not found")]
    AccountNotFound,
}

pub struct ObricV2Amm {
    key: Pubkey,
    pub state: SSTradingPair,
    current_x: u64,
    current_y: u64,
    pub x_decimals: u8,
    pub y_decimals: u8,
}

impl Adapter for ObricV2Amm {}

impl Amm for ObricV2Amm {
    fn program_id(&self) -> Pubkey {
        OBRIC_V2
    }

    fn label(&self) -> String {
        "Obric V2".to_string()
    }

    fn get_accounts_len(&self) -> usize {
        12
    }

    fn key(&self) -> Pubkey {
        return self.key;
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        return [self.state.mint_x, self.state.mint_y].to_vec();
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        if self.x_decimals == 0 && self.y_decimals == 0 {
            return [self.state.reserve_x, self.state.reserve_y, self.state.x_price_feed_id, self.state.y_price_feed_id, self.state.mint_x, self.state.mint_y].to_vec();
        } else {
            return [self.state.reserve_x, self.state.reserve_y, self.state.x_price_feed_id, self.state.y_price_feed_id].to_vec();
        }
    }

    fn update(&mut self, accounts_map: &AccountMap) -> Result<()> {
        let reserve_x_data = &mut &accounts_map.get(&self.state.reserve_x).ok_or(AmmError::AccountNotFound)?.data[..];
        let reserve_y_data = &mut &accounts_map.get(&self.state.reserve_x).ok_or(AmmError::AccountNotFound)?.data[..];
        let reserve_x_token_account = &TokenAccount::try_deserialize(reserve_x_data)?;
        let reserve_y_token_account = &TokenAccount::try_deserialize(reserve_y_data)?;
        self.current_x = reserve_x_token_account.amount;
        self.current_y = reserve_y_token_account.amount;

        if self.x_decimals == 0 && self.y_decimals == 0 {
            let mint_x_data = &mut &accounts_map.get(&self.state.mint_x).ok_or(AmmError::AccountNotFound)?.data[..];
            let min_x = &Mint::try_deserialize(mint_x_data)?;

            let mint_y_data = &mut &accounts_map.get(&self.state.mint_y).ok_or(AmmError::AccountNotFound)?.data[..];
            let min_y = &Mint::try_deserialize(mint_y_data)?;

            self.x_decimals = min_x.decimals;
            self.y_decimals = min_y.decimals;
        }

        let price_x_data = &mut &accounts_map.get(&self.state.x_price_feed_id).ok_or(AmmError::AccountNotFound)?.data[..];
        let price_y_data = &mut &accounts_map.get(&self.state.y_price_feed_id).ok_or(AmmError::AccountNotFound)?.data[..];
        let price_x_fee = &PriceFeed::try_deserialize(price_x_data)?;
        let price_y_fee = &PriceFeed::try_deserialize(price_y_data)?;
        let price_x = price_x_fee.price_normalized()?.price as u64;
        let price_y = price_y_fee.price_normalized()?.price as u64;
        self.state.update_price(price_x, price_y, self.x_decimals, self.y_decimals)?;
        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        let (output_after_fee, protocol_fee, lp_fee) = if quote_params.input_mint.eq(&self.state.mint_x) {
            self.state.quote_x_to_y(quote_params.amount, self.current_x, self.current_y)?
        } else if quote_params.input_mint.eq(&self.state.mint_y) {
            self.state.quote_y_to_x(quote_params.amount, self.current_x, self.current_y)?
        } else {
            (0u64, 0u64, 0u64)
        };
        if output_after_fee == 0 {
            Ok(Quote::default())
        } else {
            Ok(Quote { out_amount: output_after_fee, fee_amount: protocol_fee + lp_fee, fee_mint: quote_params.output_mint, ..Quote::default() })
        }
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        let state = &self.state;
        Box::new(Self {
            key: self.key,
            state: SSTradingPair {
                is_initialized: state.is_initialized,
                x_price_feed_id: state.x_price_feed_id,
                y_price_feed_id: state.y_price_feed_id,
                reserve_x: state.reserve_x,
                reserve_y: state.reserve_y,
                protocol_fee_x: state.protocol_fee_x,
                protocol_fee_y: state.protocol_fee_y,
                bump: state.bump,
                mint_x: state.mint_x,
                mint_y: state.mint_y,
                concentration: state.concentration,
                big_k: state.big_k,
                target_x: state.target_x,
                cumulative_volume: state.cumulative_volume,
                mult_x: state.mult_x,
                mult_y: state.mult_y,
                fee_millionth: state.fee_millionth,
                rebate_percentage: state.rebate_percentage,
                protocol_fee_share_thousandth: state.protocol_fee_share_thousandth,
                volume_record: state.volume_record,
                volume_time_record: state.volume_time_record,
                padding: state.padding,
            },
            current_x: self.current_x,
            current_y: self.current_y,
            x_decimals: self.x_decimals,
            y_decimals: self.y_decimals,
        })
    }

    fn from_keyed_account(keyed_account: &KeyedAccount, _: &AmmContext) -> Result<Self>
    where
        Self: Sized,
    {
        let data = &mut &keyed_account.account.data.clone()[0..];
        let ss_trading_pair = SSTradingPair::deserialize(data)?;
        Ok(Self { key: keyed_account.key, state: ss_trading_pair, current_x: 0u64, current_y: 0u64, x_decimals: 0u8, y_decimals: 0u8 })
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let (user_token_account_x, user_token_account_y, protocol_fee) = if swap_params.source_mint.eq(&self.state.mint_x) {
            (swap_params.source_token_account, swap_params.destination_token_account, self.state.protocol_fee_y)
        } else {
            (swap_params.destination_token_account, swap_params.source_token_account, self.state.protocol_fee_x)
        };

        Ok(SwapAndAccountMetas {
            swap: Swap::ObricV2,
            account_metas: vec![
                AccountMeta::new(self.key(), false),
                AccountMeta::new_readonly(self.state.mint_x, false),
                AccountMeta::new_readonly(self.state.mint_y, false),
                AccountMeta::new(self.state.reserve_x, false),
                AccountMeta::new(self.state.reserve_y, false),
                AccountMeta::new(user_token_account_x, false),
                AccountMeta::new(user_token_account_y, false),
                AccountMeta::new(protocol_fee, false),
                AccountMeta::new_readonly(self.state.x_price_feed_id, false),
                AccountMeta::new_readonly(self.state.y_price_feed_id, false),
                AccountMeta::new_readonly(swap_params.token_transfer_authority, true),
                AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
            ],
        })
    }
}
