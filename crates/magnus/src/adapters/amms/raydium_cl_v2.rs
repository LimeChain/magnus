use anchor_spl::token_interface::spl_token_metadata_interface::borsh::{BorshDeserialize, BorshSerialize};
use futures_util::StreamExt as _;
use magnus_consts::amm_raydium_cl_v2;
use solana_sdk::pubkey::Pubkey;

use crate::adapters::{Adapter, amms::Amm};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RaydiumCLV2;

impl Adapter for RaydiumCLV2 {}

impl RaydiumCLV2 {
    pub fn new() -> RaydiumCLV2 {
        RaydiumCLV2::default()
    }
}

impl Amm for RaydiumCLV2 {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str_const(&amm_raydium_cl_v2::id().to_string())
    }

    fn label(&self) -> String {
        "RaydiumConcentratedLiquidity".to_string()
    }

    fn key(&self) -> solana_sdk::pubkey::Pubkey {
        unimplemented!()
    }

    fn get_reserve_mints(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        unimplemented!()
    }

    fn get_accounts_to_update(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        unimplemented!()
    }

    fn update(&mut self, _account_map: &super::AccountMap, _: Option<u64>) -> eyre::Result<()> {
        unimplemented!()
    }

    fn quote(&self, _quote_params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::Quote> {
        unimplemented!()
    }

    fn get_swap_and_account_metas(&self, _swap_params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        unimplemented!()
    }

    fn from_keyed_account(_keyed_account: &super::KeyedAccount, _amm_context: &super::AmmContext) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        unimplemented!()
    }
}

/*
 *
 */
#[derive(Debug)]
pub struct SwapV1 {
    /// The user performing the swap
    pub payer: solana_sdk::pubkey::Pubkey,
    /// The factory state to read protocol fees
    pub amm_config: solana_sdk::pubkey::Pubkey,
    /// The program account of the pool in which the swap will be performed
    pub pool_state: solana_sdk::pubkey::Pubkey,
    /// The user token account for input token
    pub input_token_account: solana_sdk::pubkey::Pubkey,
    /// The user token account for output token
    pub output_token_account: solana_sdk::pubkey::Pubkey,
    /// The vault token account for input token
    pub input_vault: solana_sdk::pubkey::Pubkey,
    /// The vault token account for output token
    pub output_vault: solana_sdk::pubkey::Pubkey,
    /// The program account for the most recent oracle observation
    pub observation_state: solana_sdk::pubkey::Pubkey,
    /// SPL program for token transfers
    pub token_program: solana_sdk::pubkey::Pubkey,
    /// SPL program 2022 for token transfers
    pub token_program2022: solana_sdk::pubkey::Pubkey,
    /// Memo program
    pub memo_program: solana_sdk::pubkey::Pubkey,
    /// The mint of token vault 0
    pub input_vault_mint: solana_sdk::pubkey::Pubkey,
    /// The mint of token vault 1
    pub output_vault_mint: solana_sdk::pubkey::Pubkey,
}

impl SwapV1 {
    pub fn instruction(&self, args: SwapV2InstructionArgs) -> solana_instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }

    #[allow(clippy::arithmetic_side_effects)]
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(&self, args: SwapV2InstructionArgs, remaining_accounts: &[solana_instruction::AccountMeta]) -> solana_instruction::Instruction {
        let mut accounts = Vec::with_capacity(13 + remaining_accounts.len());
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.payer, true));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.amm_config, false));
        accounts.push(solana_instruction::AccountMeta::new(self.pool_state, false));
        accounts.push(solana_instruction::AccountMeta::new(self.input_token_account, false));
        accounts.push(solana_instruction::AccountMeta::new(self.output_token_account, false));
        accounts.push(solana_instruction::AccountMeta::new(self.input_vault, false));
        accounts.push(solana_instruction::AccountMeta::new(self.output_vault, false));
        accounts.push(solana_instruction::AccountMeta::new(self.observation_state, false));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.token_program, false));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.token_program2022, false));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.memo_program, false));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.input_vault_mint, false));
        accounts.push(solana_instruction::AccountMeta::new_readonly(self.output_vault_mint, false));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = SwapV1InstructionData::new().try_to_vec().unwrap();
        let mut args = args.try_to_vec().unwrap();
        data.append(&mut args);

        solana_instruction::Instruction { program_id: Pubkey::from_str_const(&amm_raydium_cl_v2::id().to_string()), accounts, data }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SwapV1InstructionData {
    discriminator: [u8; 8],
}

impl SwapV1InstructionData {
    pub fn new() -> Self {
        Self { discriminator: [43, 4, 237, 11, 26, 201, 30, 98] }
    }

    pub(crate) fn try_to_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}

impl Default for SwapV1InstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SwapV2InstructionArgs {
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: u128,
    pub is_base_input: bool,
}

impl SwapV2InstructionArgs {
    pub(crate) fn try_to_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }
}

#[derive(Debug, Clone)]
pub struct DecodedAccount<T> {
    pub address: solana_sdk::pubkey::Pubkey,
    pub account: solana_account::Account,
    pub data: T,
}

pub fn fetch_pool_state(rpc: &solana_client::rpc_client::RpcClient, address: &solana_sdk::pubkey::Pubkey) -> Result<DecodedAccount<PoolState>, std::io::Error> {
    let accounts = fetch_all_pool_state(rpc, &[*address])?;
    Ok(accounts[0].clone())
}

pub fn fetch_all_pool_state(rpc: &solana_client::rpc_client::RpcClient, addresses: &[solana_sdk::pubkey::Pubkey]) -> Result<Vec<DecodedAccount<PoolState>>, std::io::Error> {
    let accounts = rpc.get_multiple_accounts(addresses).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut decoded_accounts: Vec<DecodedAccount<PoolState>> = Vec::new();
    for i in 0..addresses.len() {
        let address = addresses[i];
        let account = accounts[i].as_ref().ok_or(std::io::Error::other(format!("Account not found: {}", address)))?;
        let data = PoolState::from_bytes(&account.data)?;
        decoded_accounts.push(DecodedAccount { address, account: account.clone(), data });
    }
    Ok(decoded_accounts)
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PoolState {
    pub discriminator: [u8; 8],
    /// Bump to identify PDA
    pub bump: [u8; 1],
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub amm_config: Pubkey,
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub owner: Pubkey,
    /// Token pair of the pool, where token_mint_0 address < token_mint_1 address
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_mint0: Pubkey,
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_mint1: Pubkey,
    /// Token pair vault
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_vault0: Pubkey,
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_vault1: Pubkey,
    /// observation account key
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub observation_key: Pubkey,
    /// mint0 and mint1 decimals
    pub mint_decimals0: u8,
    pub mint_decimals1: u8,
    /// The minimum number of ticks between initialized ticks
    pub tick_spacing: u16,
    /// The currently in range liquidity available to the pool.
    pub liquidity: u128,
    /// The current price of the pool as a sqrt(token_1/token_0) Q64.64 value
    pub sqrt_price_x64: u128,
    /// The current tick of the pool, i.e. according to the last tick transition that was run.
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    /// The fee growth as a Q64.64 number, i.e. fees of token_0 and token_1 collected per
    /// unit of liquidity for the entire life of the pool.
    pub fee_growth_global0_x64: u128,
    pub fee_growth_global1_x64: u128,
    /// The amounts of token_0 and token_1 that are owed to the protocol.
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    /// The amounts in and out of swap token_0 and token_1
    pub swap_in_amount_token0: u128,
    pub swap_out_amount_token1: u128,
    pub swap_in_amount_token1: u128,
    pub swap_out_amount_token0: u128,
    /// Bitwise representation of the state of the pool
    /// bit0, 1: disable open position and increase liquidity, 0: normal
    /// bit1, 1: disable decrease liquidity, 0: normal
    /// bit2, 1: disable collect fee, 0: normal
    /// bit3, 1: disable collect reward, 0: normal
    /// bit4, 1: disable swap, 0: normal
    pub status: u8,
    /// Leave blank for future use
    pub padding: [u8; 7],
    pub reward_infos: [RewardInfo; 3],
    /// Packed initialized tick array state
    pub tick_array_bitmap: [u64; 16],
    /// except protocol_fee and fund_fee
    pub total_fees_token0: u64,
    /// except protocol_fee and fund_fee
    pub total_fees_claimed_token0: u64,
    pub total_fees_token1: u64,
    pub total_fees_claimed_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding1: [u64; 24],
    pub padding2: [u64; 32],
}

pub const POOL_STATE_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

impl PoolState {
    pub const LEN: usize = 1544;

    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut data = data;
        Self::deserialize(&mut data)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RewardInfo {
    /// Reward state
    pub reward_state: u8,
    /// Reward open time
    pub open_time: u64,
    /// Reward end time
    pub end_time: u64,
    /// Reward last update time
    pub last_update_time: u64,
    /// Q64.64 number indicates how many tokens per second are earned per unit of liquidity.
    pub emissions_per_second_x64: u128,
    /// The total amount of reward emissioned
    pub reward_total_emissioned: u64,
    /// The total amount of claimed reward
    pub reward_claimed: u64,
    /// Reward token mint.
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_mint: Pubkey,
    /// Reward vault token account.
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub token_vault: Pubkey,
    /// The owner that has permission to set reward param
    #[serde(with = "serde_with::As::<serde_with::DisplayFromStr>")]
    pub authority: Pubkey,
    /// Q64.64 number that tracks the total tokens earned per unit of liquidity since the reward
    /// emissions were turned on.
    pub reward_growth_global_x64: u128,
}
/*
 *
 *
 *
 */
