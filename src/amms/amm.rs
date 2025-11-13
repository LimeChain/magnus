use anchor_lang::{ToAccountMetas, prelude::AccountMeta};
pub use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, AmmLabel, AmmProgramIdToLabel, KeyedAccount, KeyedUiAccount, Quote, QuoteParams, Side, SingleProgramAmm, Swap, SwapAndAccountMetas, SwapMode,
    SwapParams, single_program_amm, try_get_account_data,
};
use solana_sdk::pubkey::Pubkey;

pub fn to_dex_account_metas(program_id: anchor_lang::prelude::Pubkey, accounts: impl ToAccountMetas) -> Vec<AccountMeta> {
    let mut account_metas = vec![AccountMeta::new_readonly(program_id, false)];
    account_metas.extend(accounts.to_account_metas(None));
    account_metas
}
