use anchor_lang::prelude::*;

pub fn log_swap_basic_info(order_id: u64, source_mint: &Pubkey, destination_mint: &Pubkey, source_owner: &Pubkey, destination_owner: &Pubkey) {
    if order_id > 0 {
        msg!("order_id: {}", order_id);
    }
    source_mint.log();
    destination_mint.log();
    source_owner.log();
    destination_owner.log();
}

pub fn log_swap_balance_before(before_source_balance: u64, before_destination_balance: u64, amount_in: u64, expect_amount_out: u64, min_return: u64) {
    msg!(
        "before_source_balance: {}, before_destination_balance: {}, amount_in: {}, expect_amount_out: {}, min_return: {}",
        before_source_balance,
        before_destination_balance,
        amount_in,
        expect_amount_out,
        min_return
    );
}

pub fn log_swap_end(after_source_balance: u64, after_destination_balance: u64, source_token_change: u64, destination_token_change: u64) {
    msg!(
        "after_source_balance: {}, after_destination_balance: {}, source_token_change: {}, destination_token_change: {}",
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change
    );
}
