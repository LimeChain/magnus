use std::str::FromStr;

use eyre::eyre;
use magnus_router_client::instructions::SwapBuilder;
use magnus_shared::{Dex, Route, pmm_humidifi};
use rust_decimal::dec;
use serde::{Deserialize, Serialize};
use solana_instruction::AccountMeta;
use solana_sdk::{pubkey::Pubkey, sysvar, transaction::Transaction};

use crate::adapters::{
    Adapter, AmmKind, SwapParams,
    amms::{Amm, Chroot},
};

/*
 * Few things that might be more opaque here:
 *
 * Since we cannot directly deserialize into some structure (there's no clue how
 * humidifi, or any other prop AMM for that matter, keeps track of its state),
 * we'll simulate the `quote` and `swap` expected by the `Amm` trait through
 * a virtual env established through litesvm.
 */
//#[derive(Default)]
pub struct Humidifi {
    key: Pubkey,
    cfg: HumidifiCfg,
    chroot: Chroot,
}

impl Humidifi {
    pub fn create_humidifi_param(swap_id: u64) -> Pubkey {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&swap_id.to_le_bytes());
        Pubkey::new_from_array(bytes)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HumidifiCfg {
    pub pubkey: Pubkey,
    pub market: Pubkey,
    pub base_ta: Pubkey,
    pub quote_ta: Pubkey,
    pub reserve_mints: [(Pubkey, u8); 2],
}

impl TryFrom<&serde_json::Value> for HumidifiCfg {
    type Error = String;

    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let accounts = value.get("accounts").and_then(|v| v.as_array()).and_then(|arr| arr.first()).ok_or("missing accounts array")?;
        let pubkey = value.get("pubkey").and_then(|v| v.as_str()).ok_or("missing pubkey")?;
        let market = accounts.get("market").and_then(|v| v.as_str()).ok_or("missing market")?;
        let base_ta = accounts.get("base_ta").and_then(|v| v.as_str()).ok_or("missing base_ta")?;
        let quote_ta = accounts.get("quote_ta").and_then(|v| v.as_str()).ok_or("missing quote_ta")?;
        let reserve_mints = value.get("reserve_mints").and_then(|v| v.as_array()).ok_or("missing reserve_mints")?;

        if reserve_mints.len() != 2 {
            return Err("reserve_mints must have exactly 2 elements".to_string());
        }

        let mint0 = reserve_mints[0].as_array().ok_or("reserve_mints[0] not a string")?;
        let (mint0_addr, mint0_dec) = (mint0[0].as_str().ok_or("reserve_mints[0][0] not a string")?, mint0[1].as_u64().ok_or("reserve_mints[0][1] not a u64")?);

        let mint1 = reserve_mints[1].as_array().ok_or("reserve_mints[1] not a string")?;
        let (mint1_addr, mint1_dec) = (mint1[0].as_str().ok_or("reserve_mints[1][0] not a string")?, mint1[1].as_u64().ok_or("reserve_mints[1][1] not a u64")?);

        Ok(HumidifiCfg {
            pubkey: Pubkey::from_str(pubkey).map_err(|e| e.to_string())?,
            market: Pubkey::from_str(market).map_err(|e| e.to_string())?,
            base_ta: Pubkey::from_str(base_ta).map_err(|e| e.to_string())?,
            quote_ta: Pubkey::from_str(quote_ta).map_err(|e| e.to_string())?,
            reserve_mints: [
                (Pubkey::from_str(mint0_addr).map_err(|e| e.to_string())?, mint0_dec as u8),
                (Pubkey::from_str(mint1_addr).map_err(|e| e.to_string())?, mint1_dec as u8),
            ],
        })
    }
}

impl Adapter for Humidifi {}

impl Humidifi {
    pub fn new(cfg: HumidifiCfg) -> eyre::Result<Humidifi> {
        let chroot = Chroot::new(cfg.reserve_mints).load_program(Pubkey::from_str_const(&pmm_humidifi::id().to_string()), "./cfg/programs/")?;
        Ok(Humidifi { key: cfg.pubkey, cfg, chroot })
    }
}

impl std::fmt::Debug for Humidifi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Humidifi | key {} ", self.key))
    }
}

impl std::fmt::Display for Humidifi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Humidifi | key {} ", self.key))
    }
}

impl Amm for Humidifi {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str_const(&pmm_humidifi::id().to_string())
    }

    fn label(&self) -> String {
        self.to_string()
    }

    fn get_accounts_len(&self) -> usize {
        pmm_humidifi::ACCOUNTS_LEN
    }

    fn key(&self) -> solana_sdk::pubkey::Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.cfg.reserve_mints.map(|(addr, _)| addr).to_vec()
    }

    fn get_accounts_to_update(&self) -> Vec<solana_sdk::pubkey::Pubkey> {
        [self.key, self.cfg.market, self.cfg.base_ta, self.cfg.quote_ta].to_vec()
    }

    fn update(&mut self, account_map: &super::AccountMap, slot: Option<u64>) -> eyre::Result<()> {
        let accs = account_map.iter().map(|(key, account)| (*key, account.clone())).collect();

        self.chroot.update_accounts(accs);

        if let Some(slot) = slot {
            self.chroot.update_slot(slot);
        }

        Ok(())
    }

    fn quote(&mut self, params: &crate::adapters::QuoteParams) -> eyre::Result<crate::adapters::Quote> {
        let src_ta = Chroot::get_ta(params.input_mint, self.chroot.wallet_pubkey());
        let dst_ta = Chroot::get_ta(params.output_mint, self.chroot.wallet_pubkey());
        let routes: Vec<Vec<magnus_router_client::types::Route>> = vec![vec![Route { dexes: vec![Dex::Humidifi], weights: vec![100] }.into()]];
        let swap_params = SwapParams {
            swap_mode: params.swap_mode,
            amount: params.amount,
            input_mint: params.input_mint,
            output_mint: params.output_mint,
            src_ta,
            dst_ta,
            token_transfer_authority: self.chroot.wallet_pubkey(),
        };
        let order_id = Chroot::gen_order_id();
        let construct = self.get_swap_and_account_metas(&swap_params)?;

        let mut swap_builder = SwapBuilder::new();
        let swap = swap_builder
            .payer(self.chroot.wallet_pubkey())
            .source_token_account(src_ta)
            .destination_token_account(dst_ta)
            .source_mint(params.input_mint)
            .destination_mint(params.output_mint)
            .amount_in(params.amount)
            .expect_amount_out(1)
            .min_return(1)
            .amounts(vec![params.amount])
            .routes(routes)
            .order_id(order_id)
            .add_remaining_accounts(&construct.account_metas);

        let ix = swap.instruction();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&self.chroot.wallet_pubkey()), &[&self.chroot.wallet], self.chroot.svm.latest_blockhash());
        let res = self.chroot.svm.send_transaction(tx).map_err(|e| eyre!("{:?}", e))?;

        let amount_out = self.chroot.get_event_amount_out(&res);

        // then: reset the env amounts by nullifying the amounts of the input and output tokens

        Ok(crate::adapters::Quote { in_amount: params.amount, out_amount: amount_out, fee_amount: 0, fee_pct: dec!(0.0), fee_mint: Pubkey::new_unique() })
    }

    fn get_swap_and_account_metas(&self, params: &crate::adapters::SwapParams) -> eyre::Result<crate::adapters::SwapAndAccountMetas> {
        let kind = AmmKind::Humidifi;

        let res = crate::adapters::SwapAndAccountMetas {
            swap: kind,
            account_metas: vec![
                AccountMeta::new_readonly(Pubkey::new_from_array(pmm_humidifi::id().to_bytes()), false),
                AccountMeta::new(params.token_transfer_authority, true),
                AccountMeta::new(params.src_ta, false),
                AccountMeta::new(params.dst_ta, false),
                AccountMeta::new_readonly(Humidifi::create_humidifi_param(1500), false),
                AccountMeta::new(self.cfg.market, false),
                AccountMeta::new(self.cfg.base_ta, false),
                AccountMeta::new(self.cfg.quote_ta, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(sysvar::instructions::id(), false),
            ],
        };

        Ok(res)
    }

    fn from_keyed_account(_keyed_account: &super::KeyedAccount) -> eyre::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    //fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    //    Box::new(self.clone())
    //}
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    #[test]
//    fn test_humidifi_from_value() {
//        let json = r#"{
//            "dex": "humidifi",
//            "pubkey": "FksffEqnBRixYGR791Qw2MgdU7zNCpHVFYBL4Fa4qVuH",
//            "accounts": [
//                {
//                    "market": "FksffEqnBRixYGR791Qw2MgdU7zNCpHVFYBL4Fa4qVuH",
//                    "base_ta": "C3FzbX9n1YD2dow2dCmEv5uNyyf22Gb3TLAEqGBhw5fY",
//                    "quote_ta": "3RWFAQBRkNGq7CMGcTLK3kXDgFTe9jgMeFYqk8nHwcWh"
//                }
//            ],
//            "reserve_mints": [
//                "So11111111111111111111111111111111111111112",
//                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
//            ]
//        }"#;
//
//        let value: serde_json::Value = serde_json::from_str(json).unwrap();
//        let cfg = HumidifiCfg::try_from(&value).unwrap();
//
//        assert_eq!(cfg.market.to_string(), "FksffEqnBRixYGR791Qw2MgdU7zNCpHVFYBL4Fa4qVuH");
//        assert_eq!(cfg.base_ta.to_string(), "C3FzbX9n1YD2dow2dCmEv5uNyyf22Gb3TLAEqGBhw5fY");
//        assert_eq!(cfg.quote_ta.to_string(), "3RWFAQBRkNGq7CMGcTLK3kXDgFTe9jgMeFYqk8nHwcWh");
//        assert_eq!(cfg.reserve_mints[0].to_string(), "So11111111111111111111111111111111111111112");
//        assert_eq!(cfg.reserve_mints[1].to_string(), "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
//    }
//}
//
