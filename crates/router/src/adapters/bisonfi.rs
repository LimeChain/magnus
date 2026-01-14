use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};
use magnus_shared::pmm_goonfi::{self, ACCOUNTS_LEN, ARGS_LEN};

use super::common::DexProcessor;
use crate::{
    adapters::common::{before_check, invoke_process},
    error::ErrorCode,
    HopAccounts, GOONFI_SWAP_SELECTOR,
};

pub struct BisonfiProcessor;
impl DexProcessor for BisonfiProcessor {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub amount_in: u64,
    pub amount_out_min: u64,
    pub a_to_b: bool,
}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::BisonFi amount_in: {}, offset: {}", amount_in, offset);

    Ok(0)
}
