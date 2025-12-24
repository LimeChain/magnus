use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;
use magnus_shared::amm_raydium_cp::{self, ACCOUNTS_LEN, ARGS_LEN};

use crate::{
    adapters::common::{before_check, invoke_process, DexProcessor},
    error::ErrorCode,
    HopAccounts, CPSWAP_SELECTOR,
};

pub struct RaydiumSwapProcessor;
impl DexProcessor for RaydiumSwapProcessor {}

pub struct RaydiumCPAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub authority: &'info AccountInfo<'info>,
    pub amm_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub input_vault: InterfaceAccount<'info, TokenAccount>,
    pub output_vault: InterfaceAccount<'info, TokenAccount>,
    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub input_token_mint: InterfaceAccount<'info, Mint>,
    pub output_token_mint: InterfaceAccount<'info, Mint>,
    pub observation_state: &'info AccountInfo<'info>,
}

impl<'info> RaydiumCPAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            authority,
            amm_config,
            pool_state,
            input_vault,
            output_vault,
            input_token_program,
            output_token_program,
            input_token_mint,
            output_token_mint,
            observation_state,
        ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            authority,
            amm_config,
            pool_state,
            input_vault: InterfaceAccount::try_from(input_vault)?,
            output_vault: InterfaceAccount::try_from(output_vault)?,
            input_token_program: Interface::try_from(input_token_program)?,
            output_token_program: Interface::try_from(output_token_program)?,
            input_token_mint: InterfaceAccount::try_from(input_token_mint)?,
            output_token_mint: InterfaceAccount::try_from(output_token_mint)?,
            observation_state,
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
    msg!("Dex::RaydiumCpmmSwap amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = RaydiumCPAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &amm_raydium_cp::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // log pool address
    swap_accounts.pool_state.key().log();

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(swap_accounts.swap_authority_pubkey, &swap_accounts.swap_source_token, swap_destination_token, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let minimum_amount_out = 0u64;
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(CPSWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_config.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_token_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_token_mint.key(), false),
        AccountMeta::new(swap_accounts.observation_state.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.authority.to_account_info(),
        swap_accounts.amm_config.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.input_token_program.to_account_info(),
        swap_accounts.output_token_program.to_account_info(),
        swap_accounts.input_token_mint.to_account_info(),
        swap_accounts.output_token_mint.to_account_info(),
        swap_accounts.observation_state.to_account_info(),
    ];

    let instruction = Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_cpmm_instruction() {
        let amount_in = 100u64;
        let minimum_amount_out = 0u64;

        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(CPSWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&minimum_amount_out.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
