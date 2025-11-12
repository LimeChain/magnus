use anchor_lang::AccountDeserialize;
use solana_sdk::{account::Account, pubkey::Pubkey};

pub fn parse_amount(s: &str) -> Option<u64> {
    s.parse::<u64>().ok()
}

// Utility function to deserialize Anchor accounts
pub fn deserialize_anchor_account<T: AccountDeserialize>(account: &Account) -> eyre::Result<T> {
    let mut data: &[u8] = &account.data;
    T::try_deserialize(&mut data).map_err(Into::into)
}

pub fn geyser_acc_to_native(account_info: &yellowstone_grpc_proto::prelude::SubscribeUpdateAccountInfo) -> Account {
    Account {
        lamports: account_info.lamports,
        data: account_info.data.clone(),
        owner: Pubkey::try_from(account_info.owner.as_slice()).unwrap(),
        executable: account_info.executable,
        rent_epoch: account_info.rent_epoch,
    }
}
