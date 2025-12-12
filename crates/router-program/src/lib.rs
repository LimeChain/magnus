use anchor_lang::prelude::*;
pub mod adapters;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod processor;
pub mod utils;

pub use constants::*;
pub use instructions::*;
pub use processor::*;

declare_id!("6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma");

#[program]
pub mod router {
    use super::*;

    pub fn swap<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    pub fn swap_v3<'a>(ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>, args: SwapArgs, commission_info: u32, platform_fee_rate: u16, order_id: u64) -> Result<()> {
        instructions::swap_toc_handler(ctx, args, commission_info, order_id, Some(platform_fee_rate))
    }

    pub fn swap_tob_v3<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
        args: SwapArgs,
        commission_info: u32,
        trim_rate: u8,
        platform_fee_rate: u16,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_tob_handler(ctx, args, commission_info, order_id, Some(trim_rate), Some(platform_fee_rate))
    }
}
