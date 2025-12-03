use core::ops::Deref;

use anchor_lang::prelude::{AccountDeserialize, AccountSerialize, Error, Result, error, error_code, msg};
use borsh::{BorshDeserialize, BorshSerialize};
use num::{integer::Roots, pow};
use pyth_sdk::Price;
use pyth_sdk_solana::state::{GenericPriceAccount, load_price_account};
use solana_sdk::{pubkey, pubkey::Pubkey};

const MILLION: u64 = 1_000_000;
pub const PYTH_PROGRAM_ID: Pubkey = pubkey!("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH");

#[derive(Clone, Debug)]
pub struct PriceFeed(pyth_sdk::PriceFeed);

#[error_code]
pub enum ObricError {
    #[msg("Pyth has an internal error")]
    PythError,
    #[msg("Program should not try to serialize a price account")]
    TryToSerializePriceAccount,
}

impl PriceFeed {
    pub fn price_normalized(&self) -> Result<Price> {
        let p = self.0.get_price_unchecked();
        let price = p.scale_to_exponent(-3).ok_or(ObricError::PythError)?;
        Ok(price)
    }
}

impl AccountDeserialize for PriceFeed {
    fn try_deserialize_unchecked(data: &mut &[u8]) -> Result<Self> {
        let account: &GenericPriceAccount<32, ()> = load_price_account(data).map_err(|_x| error!(ObricError::PythError))?;

        // Use a dummy key since the key field will be removed from the SDK
        let feed = account.to_price_feed(&PYTH_PROGRAM_ID);
        return Ok(PriceFeed(feed));
    }
}

impl AccountSerialize for PriceFeed {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        Err(error!(ObricError::TryToSerializePriceAccount))
    }
}

impl Deref for PriceFeed {
    type Target = pyth_sdk::PriceFeed;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/* -- */
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct SSTradingPair {
    pub is_initialized: bool,

    pub x_price_feed_id: Pubkey,
    pub y_price_feed_id: Pubkey,

    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,

    pub protocol_fee_x: Pubkey,
    pub protocol_fee_y: Pubkey,

    pub bump: u8,
    // mints
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,

    pub concentration: u64,
    pub big_k: u128,
    pub target_x: u64,

    pub cumulative_volume: u64,

    pub mult_x: u64,
    pub mult_y: u64,
    pub fee_millionth: u64,
    pub rebate_percentage: u64,
    pub protocol_fee_share_thousandth: u64,

    pub volume_record: [u64; 8],
    pub volume_time_record: [i64; 8],

    pub padding: [u64; 24],
}

impl SSTradingPair {
    #[inline(never)]
    pub fn update_price(&mut self, price_x: u64, price_y: u64, x_decimals: u8, y_decimals: u8) -> Result<()> {
        let (x_deci_mult, y_deci_mult) = if x_decimals > y_decimals {
            (1 as u64, pow(10, usize::from(x_decimals - y_decimals)))
        } else if y_decimals > x_decimals {
            (pow(10, usize::from(y_decimals - x_decimals)), 1 as u64)
        } else {
            (1 as u64, 1 as u64)
        };

        self.mult_x = price_x * x_deci_mult;
        self.mult_y = price_y * y_deci_mult;

        Ok(())
    }

    pub fn get_target_xy(&self, current_x: u64, current_y: u64) -> Result<(u64, u64)> {
        let value_x = (current_x as u128) * (self.mult_x as u128);
        let value_y = (current_y as u128) * (self.mult_y as u128);
        let value_total = value_x + value_y;

        let target_x = self.target_x;
        let target_x_value = (target_x as u128) * (self.mult_x as u128);
        let target_y_value = value_total - target_x_value;
        let target_y = (target_y_value / (self.mult_y as u128)) as u64;
        Ok((target_x, target_y))
    }

    /**
    Returns (output_to_user, fee_to_protocol)
     */
    #[inline(never)]
    pub fn quote_x_to_y(&self, input_x: u64, current_x: u64, current_y: u64) -> Result<(u64, u64, u64)> {
        if input_x == 0 {
            return Ok((0u64, 0u64, 0u64));
        }

        let (target_x, _target_y) = self.get_target_xy(current_x, current_y)?;

        // 0. get target_x on curve-K
        let big_k = self.big_k;
        //target_x_K = sqrt(big_k / p), where p = mult_x / mult_y
        let target_x_k = (big_k * (self.mult_y as u128) / (self.mult_x as u128)).sqrt();

        // 1. find current (x,y) on curve-K
        let current_x_k = target_x_k - (target_x as u128) + (current_x as u128);
        let current_y_k = big_k / current_x_k;

        // 2. find new (x, y) on curve-K
        let new_x_k = current_x_k + (input_x as u128);
        let new_y_k = big_k / new_x_k;

        let output_before_fee_y: u64 = (current_y_k - new_y_k) as u64;
        if output_before_fee_y >= current_y {
            return Ok((0u64, 0u64, 0u64));
        }
        let fee_before_rebate_y = output_before_fee_y * self.fee_millionth / MILLION;
        let rebate_ratio = std::cmp::min(input_x, target_x - std::cmp::min(target_x, current_x)) * 100 / input_x;
        let rebate_y = fee_before_rebate_y * rebate_ratio / 100 * self.rebate_percentage / 100;
        let fee_y = fee_before_rebate_y - rebate_y;
        let output_after_fee_y = output_before_fee_y - fee_y;

        let protocol_fee_y = fee_y * self.protocol_fee_share_thousandth / 1000;
        let lp_fee_y = fee_y - protocol_fee_y;

        Ok((output_after_fee_y, protocol_fee_y, lp_fee_y))
    }

    /**
    Returns (output_to_user, fee_to_protocol, fee_to_reserve_x)
     */
    #[inline(never)]
    pub fn quote_y_to_x(&self, input_y: u64, current_x: u64, current_y: u64) -> Result<(u64, u64, u64)> {
        if input_y == 0 {
            return Ok((0u64, 0u64, 0u64));
        }

        let (target_x, target_y) = self.get_target_xy(current_x, current_y)?;

        // 0. get target_x on curve-K
        let big_k = self.big_k;
        //target_x_K = sqrt(big_k / p), where p = mult_x / mult_y
        let target_x_k = (big_k * (self.mult_y as u128) / (self.mult_x as u128)).sqrt();

        // 1. find current (x, y) on curve-K
        let current_x_k = target_x_k - (target_x as u128) + (current_x as u128);
        let current_y_k = big_k / current_x_k;

        // 2. find new (x, y) on curve-K
        let new_y_k = current_y_k + (input_y as u128);
        let new_x_k = big_k / new_y_k;

        let output_before_fee_x: u64 = (current_x_k - new_x_k) as u64;
        if output_before_fee_x >= current_x {
            return Ok((0u64, 0u64, 0u64));
        }

        let fee_before_rebate_x = output_before_fee_x * (self.fee_millionth) / MILLION;
        let rebate_ratio = std::cmp::min(input_y, target_y - std::cmp::min(target_y, current_y)) * 100 / input_y;
        let rebate_x = fee_before_rebate_x * rebate_ratio / 100 * self.rebate_percentage / 100;
        let fee_x = fee_before_rebate_x - rebate_x;
        let output_after_fee_x = output_before_fee_x - fee_x;

        let protocol_fee_x = fee_x * self.protocol_fee_share_thousandth / 1000;
        let lp_fee_x = fee_x - protocol_fee_x;

        Ok((output_after_fee_x, protocol_fee_x, lp_fee_x))
    }
}
