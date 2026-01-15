use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};
use magnus_shared::pmm_bisonfi::{self, ACCOUNTS_LEN, ARGS_LEN};

use super::common::DexProcessor;
use crate::{
    adapters::common::{before_check, invoke_process},
    error::ErrorCode,
    HopAccounts, BISONFI_SWAP_SELECTOR,
};

pub struct BisonfiProcessor;
impl DexProcessor for BisonfiProcessor {}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub amount_in: u64,
    pub amount_out_min: u64,
    pub b_to_a: bool,
}

pub struct BisonfiAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub market_base_ta: InterfaceAccount<'info, TokenAccount>,
    pub market_quote_ta: InterfaceAccount<'info, TokenAccount>,
    pub swap_src_ta: InterfaceAccount<'info, TokenAccount>,
    pub swap_dst_ta: InterfaceAccount<'info, TokenAccount>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}

impl<'info> BisonfiAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            market,
            market_base_ta,
            market_quote_ta,
            swap_src_ta,
            swap_dst_ta,
            base_token_program,
            quote_token_program,
            sysvar_instructions,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            market,
            market_base_ta: InterfaceAccount::try_from(market_base_ta)?,
            market_quote_ta: InterfaceAccount::try_from(market_quote_ta)?,
            swap_src_ta: InterfaceAccount::try_from(swap_src_ta)?,
            swap_dst_ta: InterfaceAccount::try_from(swap_dst_ta)?,
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            sysvar_instructions,
        })
    }
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

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = BisonfiAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &pmm_bisonfi::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.market.key().log();

    before_check(swap_accounts.swap_authority, &swap_accounts.swap_src_ta, swap_accounts.swap_dst_ta.key(), hop_accounts, hop, proxy_swap, owner_seeds)?;

    let b_to_a = swap_accounts.swap_src_ta.mint == swap_accounts.market_base_ta.mint;
    let (user_base_ta, user_quote_ta) = if b_to_a {
        (&swap_accounts.swap_dst_ta, &swap_accounts.swap_src_ta)
    } else {
        (&swap_accounts.swap_src_ta, &swap_accounts.swap_dst_ta)
    };

    let swap_params = SwapParams { amount_in, amount_out_min: 0, b_to_a };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(&[BISONFI_SWAP_SELECTOR]);
    data.extend_from_slice(&swap_params.try_to_vec()?);

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority.key(), true),
        AccountMeta::new(swap_accounts.market.key(), false),
        AccountMeta::new(swap_accounts.market_quote_ta.key(), false),
        AccountMeta::new(swap_accounts.market_base_ta.key(), false),
        AccountMeta::new(user_base_ta.key(), false),
        AccountMeta::new(user_quote_ta.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.market_quote_ta.to_account_info(),
        swap_accounts.market_base_ta.to_account_info(),
        user_base_ta.to_account_info(),
        user_quote_ta.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction = Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let amount_out = invoke_process(
        amount_in,
        &BisonfiProcessor,
        &account_infos,
        &mut swap_accounts.swap_src_ta,
        &mut swap_accounts.swap_dst_ta,
        hop_accounts,
        instruction,
        hop,
        offset,
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
