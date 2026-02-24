#![allow(unexpected_cfgs, clippy::too_many_arguments)]

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

declare_id!("F9Z9WiieTtL4giMt3eBFEsB5vmAFotLz37FWC7NcbzpT");

#[program]
pub mod router {
    use super::*;

    pub fn swap<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    // jup
    pub fn route_v2<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    // dflow
    pub fn swap2<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    // titan
    pub fn swap_route_v2<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    // okx
    pub fn swap_v3_with_cpi_event<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }
}
