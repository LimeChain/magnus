use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use borsh::BorshDeserialize;
use eyre::Result;
use magnus_shared::{pmm_obric_v2, spl_token};
use solana_instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use crate::adapters::{
    Adapter, AmmKind,
    amms::{
        AccountMap, Amm, KeyedAccount, Quote, QuoteParams, SwapAndAccountMetas, SwapParams,
        obric_v2::state::{PriceFeed, SSTradingPair},
    },
};

#[derive(thiserror::Error, Debug)]
pub enum AmmError {
    #[error("Account not found")]
    AccountNotFound,
}

#[derive(Clone, Debug, Default)]
pub struct ObricV2 {
    key: Pubkey,
    pub state: SSTradingPair,
    current_x: u64,
    current_y: u64,
    pub x_decimals: u8,
    pub y_decimals: u8,
}

impl ObricV2 {
    pub fn new() -> ObricV2 {
        ObricV2::default()
    }
}

impl Adapter for ObricV2 {}

impl Amm for ObricV2 {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str_const(&pmm_obric_v2::id().to_string())
    }

    fn label(&self) -> String {
        "ObricV2".to_string()
    }

    fn get_accounts_len(&self) -> usize {
        pmm_obric_v2::ACCOUNTS_LEN
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![self.state.mint_x, self.state.mint_y]
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        if self.x_decimals == 0 && self.y_decimals == 0 {
            [self.state.reserve_x, self.state.reserve_y, self.state.x_price_feed_id, self.state.y_price_feed_id, self.state.mint_x, self.state.mint_y].to_vec()
        } else {
            [self.state.reserve_x, self.state.reserve_y, self.state.x_price_feed_id, self.state.y_price_feed_id].to_vec()
        }
    }

    fn update(&mut self, accounts_map: &AccountMap, _: Option<u64>) -> Result<()> {
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

        let price_x_data = &mut &accounts_map.get(&self.state.x_price_feed_id).ok_or(AmmError::AccountNotFound)?.data[8..];
        let price_y_data = &mut &accounts_map.get(&self.state.y_price_feed_id).ok_or(AmmError::AccountNotFound)?.data[8..];
        let price_x_fee = &PriceFeed::try_deserialize(price_x_data)?;
        let price_y_fee = &PriceFeed::try_deserialize(price_y_data)?;
        let price_x = price_x_fee.price_normalized()?.price as u64;
        let price_y = price_y_fee.price_normalized()?.price as u64;
        self.state.update_price(price_x, price_y, self.x_decimals, self.y_decimals)?;
        Ok(())
    }

    fn quote(&mut self, quote_params: &QuoteParams) -> Result<Quote> {
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

    //fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    //    let state = &self.state;
    //    Box::new(Self {
    //        key: self.key,
    //        state: SSTradingPair {
    //            is_initialized: state.is_initialized,
    //            x_price_feed_id: state.x_price_feed_id,
    //            y_price_feed_id: state.y_price_feed_id,
    //            reserve_x: state.reserve_x,
    //            reserve_y: state.reserve_y,
    //            protocol_fee_x: state.protocol_fee_x,
    //            protocol_fee_y: state.protocol_fee_y,
    //            bump: state.bump,
    //            mint_x: state.mint_x,
    //            mint_y: state.mint_y,
    //            concentration: state.concentration,
    //            big_k: state.big_k,
    //            target_x: state.target_x,
    //            cumulative_volume: state.cumulative_volume,
    //            mult_x: state.mult_x,
    //            mult_y: state.mult_y,
    //            fee_millionth: state.fee_millionth,
    //            rebate_percentage: state.rebate_percentage,
    //            protocol_fee_share_thousandth: state.protocol_fee_share_thousandth,
    //            volume_record: state.volume_record,
    //            volume_time_record: state.volume_time_record,
    //            version: state.version,
    //            padding: state.padding,
    //            mint_sslp_x: state.mint_sslp_x,
    //            mint_sslp_y: state.mint_sslp_y,
    //            padding2: state.padding2,
    //        },
    //        current_x: self.current_x,
    //        current_y: self.current_y,
    //        x_decimals: self.x_decimals,
    //        y_decimals: self.y_decimals,
    //    })
    //}

    fn from_keyed_account(keyed_account: &KeyedAccount) -> Result<Self>
    where
        Self: Sized,
    {
        let data = &mut &keyed_account.account.data.clone()[8..];
        let ss_trading_pair = SSTradingPair::deserialize(data)?;
        Ok(Self { key: keyed_account.key, state: ss_trading_pair, current_x: 0u64, current_y: 0u64, x_decimals: 0u8, y_decimals: 0u8 })
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let (user_token_account_x, user_token_account_y, protocol_fee) = if swap_params.input_mint.eq(&self.state.mint_x) {
            (swap_params.src_ta, swap_params.dst_ta, self.state.protocol_fee_y)
        } else {
            (swap_params.dst_ta, swap_params.src_ta, self.state.protocol_fee_x)
        };

        Ok(SwapAndAccountMetas {
            swap: AmmKind::ObricV2,
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
                AccountMeta::new_readonly(Pubkey::from_str_const(&spl_token::id().to_string()), false),
            ],
        })
    }
}
