use crate::adapters::{
    Adapter,
    amms::{Amm, HUMIDIFI},
};

/*
 * Few things that might be more opaque here:
 *
 * Since we cannot directly deserialize into some structure (there's no clue how
 * humidifi, or any other prop AMM for that matter, keeps track of its state),
 * we'll simulate the `quote` and `swap` expected by the `Amm` trait through
 * a virtual env established through litesvm.
 */
pub struct Humidifi;

impl Adapter for Humidifi {}

impl Amm for Humidifi {
    fn program_id(&self) -> solana_sdk::pubkey::Pubkey {
        HUMIDIFI
    }

    fn label(&self) -> String {
        "HumidiFi".to_string()
    }
}
