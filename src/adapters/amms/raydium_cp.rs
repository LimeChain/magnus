use std::ops::{Deref, DerefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;

pub mod integration;
pub mod state;

pub use integration::RaydiumCP;

//use crate::adapters::{
//    Adapter,
//    amms::{Amm, AmmContext, BaseConstantProductAmm, KeyedAccount, RAYDIUM_CP, swap_state::ConstantProductSwapV1},
//};
//
////#[derive(Clone, Debug)]
////pub struct RaydiumCP(BaseConstantProductAmm);
//
//#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
//pub struct RaydiumCP {
//    pub amm_config: Pubkey,
//    pub pool_creator: Pubkey,
//    pub token_0_vault: Pubkey,
//    pub token_1_vault: Pubkey,
//    pub lp_mint: Pubkey,
//    pub token_0_mint: Pubkey,
//    pub token_1_mint: Pubkey,
//    pub token_0_program: Pubkey,
//    pub token_1_program: Pubkey,
//    pub observation_key: Pubkey,
//    pub auth_bump: u8,
//    pub status: u8,
//    pub lp_mint_decimals: u8,
//    pub mint_0_decimals: u8,
//    pub mint_1_decimals: u8,
//    pub lp_supply: u64,
//    pub protocol_fees_token_0: u64,
//    pub protocol_fees_token_1: u64,
//    pub fund_fees_token_0: u64,
//    pub fund_fees_token_1: u64,
//    pub open_time: u64,
//    pub recent_epoch: u64,
//    pub creator_fee_on: u8,
//    pub enable_creator_fee: bool,
//    pub padding1: [u8; 6],
//    pub creator_fees_token_0: u64,
//    pub creator_fees_token_1: u64,
//    pub padding: [u64; 28],
//}
//
//impl RaydiumCP {
//    pub fn new() -> RaydiumCP {
//        RaydiumCP::default()
//    }
//}
//
////impl Default for RaydiumCP {
////    fn default() -> RaydiumCP {
////        RaydiumCP(BaseConstantProductAmm { program_id: RAYDIUM_CP, ..BaseConstantProductAmm::default() })
////    }
////}
////
////impl Deref for RaydiumCP {
////    type Target = BaseConstantProductAmm;
////
////    fn deref(&self) -> &Self::Target {
////        &self.0
////    }
////}
////
////impl DerefMut for RaydiumCP {
////    fn deref_mut(&mut self) -> &mut Self::Target {
////        &mut self.0
////    }
////}
//
//impl Adapter for RaydiumCP {}
//
//impl Amm for RaydiumCP {
//    fn label(&self) -> String {
//        self.0.label()
//    }
//
//    fn program_id(&self) -> Pubkey {
//        (*self).program_id
//    }
//
//    fn key(&self) -> solana_sdk::pubkey::Pubkey {
//        self.0.key()
//    }
//
//    fn get_accounts_len(&self) -> usize {
//        self.0.get_accounts_len()
//    }
//
//    fn get_reserve_mints(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
//        self.0.get_reserve_mints()
//    }
//
//    fn get_accounts_to_update(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
//        self.0.get_accounts_to_update()
//    }
//
//    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
//        Box::new(RaydiumCP(self.0.clone()))
//    }
//
//    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> eyre::Result<Self> {
//        let state = ConstantProductSwapV1::unpack(&keyed_account.account.data[8..])?;
//        let reserve_mints = [state.token_a_mint, state.token_b_mint];
//
//        let label = "RaydiumConstantProduct".to_string();
//
//        let program_id = keyed_account.account.owner;
//        Ok(RaydiumCP(BaseConstantProductAmm {
//            key: keyed_account.key,
//            authority: Pubkey::find_program_address(&[&keyed_account.key.to_bytes()], &program_id).0,
//            label,
//            state,
//            reserve_mints,
//            program_id,
//            reserves: Default::default(),
//        }))
//    }
//
//    fn update(&mut self, account_map: &super::AccountMap) -> eyre::Result<()> {
//        self.0.update(account_map)
//    }
//
//    fn quote(&self, quote_params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::Quote> {
//        self.0.quote(quote_params)
//    }
//
//    fn get_swap_and_account_metas(&self, swap_params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
//        self.0.get_swap_and_account_metas(swap_params)
//    }
//}
//
