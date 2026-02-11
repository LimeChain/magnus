use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use magnus_shared::pmm_alphaq::{self, ACCOUNTS_LEN, ARGS_LEN};

use super::common::DexProcessor;
use crate::{
    adapters::common::{before_check, invoke_process},
    error::ErrorCode,
    HopAccounts, ALPHAQ_SWAP_SELECTOR, ZERO_ADDRESS,
};

pub struct AlphaqProcessor;
impl DexProcessor for AlphaqProcessor {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Direction {
    QuoteToBase,
    BaseToQuote,
}

pub struct AlphaqAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub market_param: &'info AccountInfo<'info>,
    pub user_base_ta: InterfaceAccount<'info, TokenAccount>,
    pub user_quote_ta: InterfaceAccount<'info, TokenAccount>,
    pub market_base_ta: InterfaceAccount<'info, TokenAccount>,
    pub market_quote_ta: InterfaceAccount<'info, TokenAccount>,
    pub market_base_aux: &'info AccountInfo<'info>,
    pub market_quote_aux: &'info AccountInfo<'info>,
    pub market_quote_aux_2: &'info AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}

impl<'info> AlphaqAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            market,
            market_param,
            user_base_ta,
            user_quote_ta,
            market_base_ta,
            market_quote_ta,
            market_base_aux,
            market_quote_aux,
            market_quote_aux_2,
            token_program,
            sysvar_instructions,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            market,
            market_param,
            user_base_ta: InterfaceAccount::try_from(user_base_ta)?,
            user_quote_ta: InterfaceAccount::try_from(user_quote_ta)?,
            market_base_ta: InterfaceAccount::try_from(market_base_ta)?,
            market_quote_ta: InterfaceAccount::try_from(market_quote_ta)?,
            market_base_aux,
            market_quote_aux,
            market_quote_aux_2,
            token_program: Interface::try_from(token_program)?,
            sysvar_instructions,
        })
    }
}

fn infer_direction(accounts: &AlphaqAccounts, amount_in: u64, hop_accounts: &HopAccounts) -> Result<Direction> {
    if hop_accounts.from_account != ZERO_ADDRESS {
        if hop_accounts.from_account == accounts.user_base_ta.key() {
            return Ok(Direction::BaseToQuote);
        }
        if hop_accounts.from_account == accounts.user_quote_ta.key() {
            return Ok(Direction::QuoteToBase);
        }
    }

    if hop_accounts.last_to_account != ZERO_ADDRESS {
        if hop_accounts.last_to_account == accounts.user_base_ta.key() {
            return Ok(Direction::BaseToQuote);
        }
        if hop_accounts.last_to_account == accounts.user_quote_ta.key() {
            return Ok(Direction::QuoteToBase);
        }
    }

    if hop_accounts.to_account != ZERO_ADDRESS {
        if hop_accounts.to_account == accounts.user_quote_ta.key() {
            return Ok(Direction::BaseToQuote);
        }
        if hop_accounts.to_account == accounts.user_base_ta.key() {
            return Ok(Direction::QuoteToBase);
        }
    }

    let base_can_fund = accounts.user_base_ta.amount >= amount_in;
    let quote_can_fund = accounts.user_quote_ta.amount >= amount_in;
    match (base_can_fund, quote_can_fund) {
        (true, false) => Ok(Direction::BaseToQuote),
        (false, true) => Ok(Direction::QuoteToBase),
        // Fall back to base->quote when both can fund (ambiguous without route-level mint context).
        (true, true) => Ok(Direction::BaseToQuote),
        (false, false) => Err(ErrorCode::InvalidTokenAccount.into()),
    }
}

fn build_swap_data(direction: Direction, amount_in: u64, amount_out_min: u64) -> Vec<u8> {
    let side = match direction {
        Direction::QuoteToBase => 0u8,
        Direction::BaseToQuote => 1u8,
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(&[ALPHAQ_SWAP_SELECTOR, side]);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&amount_out_min.to_le_bytes());
    data
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
    msg!("Dex::AlphaQ amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = AlphaqAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pmm_alphaq::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.market.key().log();

    let direction = infer_direction(&swap_accounts, amount_in, hop_accounts)?;
    let (swap_source_key, swap_destination_key) = match direction {
        Direction::BaseToQuote => (swap_accounts.user_base_ta.key(), swap_accounts.user_quote_ta.key()),
        Direction::QuoteToBase => (swap_accounts.user_quote_ta.key(), swap_accounts.user_base_ta.key()),
    };
    let swap_source_ta_ref = match direction {
        Direction::BaseToQuote => &swap_accounts.user_base_ta,
        Direction::QuoteToBase => &swap_accounts.user_quote_ta,
    };

    before_check(swap_accounts.swap_authority, swap_source_ta_ref, swap_destination_key, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let data = build_swap_data(direction, amount_in, 0);
    require!(data.len() == ARGS_LEN, ErrorCode::InvalidBundleInput);

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.swap_authority.key(), true),
        AccountMeta::new_readonly(swap_accounts.market.key(), false),
        AccountMeta::new(swap_accounts.market_param.key(), false),
        AccountMeta::new(swap_accounts.user_base_ta.key(), false),
        AccountMeta::new(swap_accounts.user_quote_ta.key(), false),
        AccountMeta::new(swap_accounts.market_base_ta.key(), false),
        AccountMeta::new(swap_accounts.market_quote_ta.key(), false),
        AccountMeta::new_readonly(swap_accounts.market_base_aux.key(), false),
        AccountMeta::new_readonly(swap_accounts.market_quote_aux.key(), false),
        AccountMeta::new_readonly(swap_accounts.market_quote_aux_2.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.market_param.to_account_info(),
        swap_accounts.user_base_ta.to_account_info(),
        swap_accounts.user_quote_ta.to_account_info(),
        swap_accounts.market_base_ta.to_account_info(),
        swap_accounts.market_quote_ta.to_account_info(),
        swap_accounts.market_base_aux.to_account_info(),
        swap_accounts.market_quote_aux.to_account_info(),
        swap_accounts.market_quote_aux_2.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction = Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };
    let dex_processor = &AlphaqProcessor;

    let amount_out = match direction {
        Direction::BaseToQuote => invoke_process(
            amount_in,
            dex_processor,
            &account_infos,
            &mut swap_accounts.user_base_ta,
            &mut swap_accounts.user_quote_ta,
            hop_accounts,
            instruction,
            hop,
            offset,
            ACCOUNTS_LEN,
            proxy_swap,
            owner_seeds,
        )?,
        Direction::QuoteToBase => invoke_process(
            amount_in,
            dex_processor,
            &account_infos,
            &mut swap_accounts.user_quote_ta,
            &mut swap_accounts.user_base_ta,
            hop_accounts,
            instruction,
            hop,
            offset,
            ACCOUNTS_LEN,
            proxy_swap,
            owner_seeds,
        )?,
    };

    // Keep the explicit association visible in logs for debugging ambiguous routes.
    msg!("AlphaQ direction: {:?}, source: {}, destination: {}", direction, swap_source_key, swap_destination_key);

    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::{build_swap_data, Direction};

    #[test]
    fn alphaq_swap_data_layout_matches_sample_quote_to_base() {
        let data = build_swap_data(Direction::QuoteToBase, 511_933_129, 0);
        assert_eq!(data, vec![0x0c, 0x00, 0xc9, 0x7a, 0x83, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn alphaq_swap_data_layout_matches_sample_base_to_quote() {
        let data = build_swap_data(Direction::BaseToQuote, 6_039_648_624, 0);
        assert_eq!(data, vec![0x0c, 0x01, 0x70, 0xb9, 0xfd, 0x67, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }
}
