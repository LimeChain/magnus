use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{raydium_swap_program, HopAccounts};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ACCOUNTS_LEN: usize = 19;
const ARGS_LEN: usize = 17;

pub struct RaydiumSwapProcessor;
impl DexProcessor for RaydiumSwapProcessor {}

pub struct RaydiumSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub amm_id: &'info AccountInfo<'info>,
    pub amm_authority: &'info AccountInfo<'info>,
    pub amm_open_orders: &'info AccountInfo<'info>,
    pub amm_target_orders: &'info AccountInfo<'info>,
    pub pool_coin_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_pc_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_program_id: &'info AccountInfo<'info>,
    pub serum_market: &'info AccountInfo<'info>,
    pub serum_bids: &'info AccountInfo<'info>,
    pub serum_asks: &'info AccountInfo<'info>,
    pub serum_event_queue: &'info AccountInfo<'info>,
    pub serum_coin_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_pc_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_vault_signer: &'info AccountInfo<'info>,
}

impl<'info> RaydiumSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_coin_token_account,
            pool_pc_token_account,
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer,
        ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program: Program::try_from(token_program)?,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_coin_token_account: Box::new(InterfaceAccount::try_from(pool_coin_token_account)?),
            pool_pc_token_account: Box::new(InterfaceAccount::try_from(pool_pc_token_account)?),
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account: Box::new(InterfaceAccount::try_from(
                serum_coin_vault_account,
            )?),
            serum_pc_vault_account: Box::new(InterfaceAccount::try_from(serum_pc_vault_account)?),
            serum_vault_signer,
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
    msg!(
        "Dex::RaydiumSwap amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_swap_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.amm_id.key().log();

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(9);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        // amm
        AccountMeta::new(swap_accounts.amm_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_authority.key(), false),
        AccountMeta::new(swap_accounts.amm_open_orders.key(), false),
        AccountMeta::new(swap_accounts.amm_target_orders.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_account.key(), false),
        // serum
        AccountMeta::new_readonly(swap_accounts.serum_program_id.key(), false),
        AccountMeta::new(swap_accounts.serum_market.key(), false),
        AccountMeta::new(swap_accounts.serum_bids.key(), false),
        AccountMeta::new(swap_accounts.serum_asks.key(), false),
        AccountMeta::new(swap_accounts.serum_event_queue.key(), false),
        AccountMeta::new(swap_accounts.serum_coin_vault_account.key(), false),
        AccountMeta::new(swap_accounts.serum_pc_vault_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.serum_vault_signer.key(), false),
        // user
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
    ];

    let account_infos = vec![
        swap_accounts.token_program.to_account_info(),
        swap_accounts.amm_id.to_account_info(),
        swap_accounts.amm_authority.to_account_info(),
        swap_accounts.amm_open_orders.to_account_info(),
        swap_accounts.amm_target_orders.to_account_info(),
        swap_accounts.pool_coin_token_account.to_account_info(),
        swap_accounts.pool_pc_token_account.to_account_info(),
        swap_accounts.serum_program_id.to_account_info(),
        swap_accounts.serum_market.to_account_info(),
        swap_accounts.serum_bids.to_account_info(),
        swap_accounts.serum_asks.to_account_info(),
        swap_accounts.serum_event_queue.to_account_info(),
        swap_accounts.serum_coin_vault_account.to_account_info(),
        swap_accounts.serum_pc_vault_account.to_account_info(),
        swap_accounts.serum_vault_signer.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

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
    pub fn test_pack_swap_instruction() {
        let amount_in = 100u64;
        let mut data = Vec::with_capacity(ARGS_LEN);
        data.push(9);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
