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

declare_id!("H3SocRxYoShPJ9z184k9tomq2GPViDKEghyYEr1SGrXE");

#[program]
pub mod router {
    use super::*;

    pub fn swap<'a>(ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>, data: SwapArgs, order_id: u64) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }
}
