use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, Token2022, TokenAccount},
};
use arrayref::array_ref;
use magnus_shared::amm_raydium_cl_v2::{self, ACCOUNTS_LEN, ARGS_LEN};

use crate::{
    adapters::{
        common::{before_check, invoke_process},
        raydium_cp::RaydiumSwapProcessor,
    },
    error::ErrorCode,
    HopAccounts, SWAPV2_SELECTOR, ZERO_ADDRESS,
};

pub struct RaydiumCLV2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub amm_config_id: &'info AccountInfo<'info>,
    pub pool_id: &'info AccountInfo<'info>,
    pub input_vault: InterfaceAccount<'info, TokenAccount>,
    pub output_vault: InterfaceAccount<'info, TokenAccount>,
    pub observation_id: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub memo_program: &'info AccountInfo<'info>,
    pub input_vault_mint: InterfaceAccount<'info, Mint>,
    pub output_vault_mint: InterfaceAccount<'info, Mint>,
    pub ex_bitmap: &'info AccountInfo<'info>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
}

impl<'info> RaydiumCLV2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            amm_config_id,
            pool_id,
            input_vault,
            output_vault,
            observation_id,
            token_program,
            token_program_2022,
            memo_program,
            input_vault_mint,
            output_vault_mint,
            ex_bitmap,
            tick_array0,
            tick_array1,
            tick_array2,

      ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            amm_config_id,
            pool_id,
            input_vault: InterfaceAccount::try_from(input_vault)?,
            output_vault: InterfaceAccount::try_from(output_vault)?,
            observation_id,
            token_program: Program::try_from(token_program)?,
            token_program_2022: Program::try_from(token_program_2022)?,
            memo_program,
            input_vault_mint: InterfaceAccount::try_from(input_vault_mint)?,
            output_vault_mint: InterfaceAccount::try_from(output_vault_mint)?,
            ex_bitmap,
            tick_array0,
            tick_array1,
            tick_array2,
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
    msg!("Dex::RaydiumClmmSwapV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = RaydiumCLV2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &amm_raydium_cl_v2::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // log pool address
    swap_accounts.pool_id.key().log();

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(swap_accounts.swap_authority_pubkey, &swap_accounts.swap_source_token, swap_destination_token, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let is_base_input = true;
    let sqrt_price_limit_x64 = 0u128;
    let other_amount_threshold = 1u64;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAPV2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.extend_from_slice(&(is_base_input as u8).to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true), // payer
        AccountMeta::new_readonly(swap_accounts.amm_config_id.key(), false),
        AccountMeta::new(swap_accounts.pool_id.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new(swap_accounts.observation_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),      // spl token
        AccountMeta::new_readonly(swap_accounts.token_program_2022.key(), false), // token 2022
        AccountMeta::new_readonly(swap_accounts.memo_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_vault_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_vault_mint.key(), false),
        AccountMeta::new(swap_accounts.ex_bitmap.key(), false),
        AccountMeta::new(swap_accounts.tick_array0.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.amm_config_id.to_account_info(),
        swap_accounts.pool_id.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.observation_id.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_program_2022.to_account_info(),
        swap_accounts.memo_program.to_account_info(),
        swap_accounts.input_vault_mint.to_account_info(),
        swap_accounts.output_vault_mint.to_account_info(),
        swap_accounts.ex_bitmap.to_account_info(),
        swap_accounts.tick_array0.to_account_info(),
    ];

    let tick_array1 = swap_accounts.tick_array1.key();
    let tick_array2 = swap_accounts.tick_array2.key();
    if tick_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array1, false));
        account_infos.push(swap_accounts.tick_array1.to_account_info());
    }
    if tick_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array2, false));
        account_infos.push(swap_accounts.tick_array2.to_account_info());
    }

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
    use crate::SWAP_SELECTOR;

    #[test]
    pub fn test_pack_clmm_instruction() {
        let amount_in = 100u64;
        let is_base_input = true;
        let sqrt_price_limit_x64 = 0u128;
        let other_amount_threshold = 1u64;

        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&other_amount_threshold.to_le_bytes());
        data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data.extend_from_slice(&(is_base_input as u8).to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
