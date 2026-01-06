use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use magnus_shared::pmm_solfi_v2::{self, ACCOUNTS_LEN, ARGS_LEN};

use super::common::DexProcessor;
use crate::{adapters::common::invoke_process, error::ErrorCode, HopAccounts};

pub struct SolfiProcessor;
impl DexProcessor for SolfiProcessor {}

pub struct SolfiAccountV2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub market: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub global_config_account: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub instruction_sysvar: &'info AccountInfo<'info>,
}

impl<'info> SolfiAccountV2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            market,
            oracle,
            global_config_account,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            instruction_sysvar,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            market,
            oracle,
            global_config_account,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            base_mint,
            quote_mint,
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            instruction_sysvar,
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
    msg!("Dex::SolfiV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = SolfiAccountV2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pmm_solfi_v2::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.market.key().log();

    let (direction, user_base_token_account, user_quote_token_account) =
        if swap_accounts.swap_source_token.mint == swap_accounts.base_mint.key() && swap_accounts.swap_destination_token.mint == swap_accounts.quote_mint.key() {
            (0u8, swap_accounts.swap_source_token.clone(), swap_accounts.swap_destination_token.clone())
        } else if swap_accounts.swap_source_token.mint == swap_accounts.quote_mint.key() && swap_accounts.swap_destination_token.mint == swap_accounts.base_mint.key() {
            (1u8, swap_accounts.swap_destination_token.clone(), swap_accounts.swap_source_token.clone())
        } else {
            return Err(ErrorCode::InvalidTokenMint.into());
        };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(7u8); //discriminator
    data.extend_from_slice(&amount_in.to_le_bytes()); //amount_in
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&direction.to_le_bytes()); //swap direction

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.market.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle.key(), false),
        AccountMeta::new_readonly(swap_accounts.global_config_account.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new(user_base_token_account.key(), false),
        AccountMeta::new(user_quote_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.instruction_sysvar.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.oracle.to_account_info(),
        swap_accounts.global_config_account.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        user_base_token_account.to_account_info(),
        user_quote_token_account.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.instruction_sysvar.to_account_info(),
    ];

    let instruction = Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &SolfiProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
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
