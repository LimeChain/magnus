//! Holds some of the shared structures/constants used across the router and the offchain service(s)
//!
//! MMs:
//! - the program id is used to indicate the route
//! - accounts_len - the number of accounts required (and expected) for successful swap at a particular exchange
//! - args_len - the number of bytes expected as instruction data

use std::str::FromStr;

use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize, borsh};

pub mod amm_raydium_cp {
    use anchor_lang::prelude::*;

    declare_id!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    pub const ACCOUNTS_LEN: usize = 14;
    pub const ARGS_LEN: usize = 24;
}

pub mod amm_raydium_cl_v2 {
    use anchor_lang::prelude::*;

    declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
    pub const ACCOUNTS_LEN: usize = 18;
    pub const ARGS_LEN: usize = 41;
}

pub mod pmm_solfi_v2 {
    use anchor_lang::prelude::*;

    declare_id!("SV2EYYJyRz2YhfXwXnhNAevDEui5Q6yrfyo13WtupPF");
    pub const ACCOUNTS_LEN: usize = 14;
    pub const ARGS_LEN: usize = 18;
}

pub mod pmm_obric_v2 {
    use anchor_lang::prelude::*;

    declare_id!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
    pub const ACCOUNTS_LEN: usize = 13;
    pub const ARGS_LEN: usize = 25;
}

pub mod pmm_humidifi {
    use anchor_lang::prelude::*;

    declare_id!("9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp");
    pub const ACCOUNTS_LEN: usize = 11;
    pub const ARGS_LEN: usize = 25;
}

pub mod pmm_zerofi {
    use anchor_lang::prelude::*;

    declare_id!("ZERor4xhbUycZ6gb9ntrhqscUcZmAbQDjEAtCf4hbZY");
    pub const ACCOUNTS_LEN: usize = 11;
    pub const ARGS_LEN: usize = 17;
}

pub mod pmm_tesserav {
    use anchor_lang::prelude::*;

    declare_id!("TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH");
    pub const ACCOUNTS_LEN: usize = 13;
    pub const ARGS_LEN: usize = 18;
}

pub mod pmm_goonfi {
    use anchor_lang::prelude::*;

    declare_id!("goonERTdGsjnkZqWuVjs73BZ3Pb9qoCUdBUL17BnS5j");
    pub const ACCOUNTS_LEN: usize = 11;
    pub const ARGS_LEN: usize = 19;
}

pub mod spl_token {
    use anchor_lang::prelude::*;

    declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
}

pub mod authority_pda {
    use anchor_lang::prelude::*;

    declare_id!("HV1KXxWFaSeriyFvXyx48FqG9BoFbfinB8njCJonqP7K");
}

pub mod system_program {
    use anchor_lang::prelude::*;

    declare_id!("11111111111111111111111111111111");
}

pub mod token_2022_program {
    use anchor_lang::prelude::*;

    declare_id!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
}

pub mod wsol_program {
    use anchor_lang::prelude::*;

    declare_id!("So11111111111111111111111111111111111111112");
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Dex {
    RaydiumClV2,
    RaydiumCp,
    ObricV2,
    SolfiV2,
    Zerofi,
    Humidifi,
}

impl std::fmt::Display for Dex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dex::RaydiumClV2 => f.write_str("raydium-cl-v2"),
            Dex::RaydiumCp => f.write_str("raydium-cp"),

            // pmms
            Dex::ObricV2 => f.write_str("obric-v2"),
            Dex::SolfiV2 => f.write_str("solfi-v2"),
            Dex::Zerofi => f.write_str("zerofi"),
            Dex::Humidifi => f.write_str("humidifi"),
        }
    }
}

impl Dex {
    pub const ALL: [Dex; 6] = [Dex::RaydiumClV2, Dex::RaydiumCp, Dex::ObricV2, Dex::SolfiV2, Dex::Zerofi, Dex::Humidifi];
    pub const PMM: [Dex; 4] = [Dex::ObricV2, Dex::SolfiV2, Dex::Zerofi, Dex::Humidifi];

    pub fn program_id(&self) -> anchor_lang::solana_program::pubkey::Pubkey {
        match self {
            Dex::RaydiumClV2 => crate::amm_raydium_cl_v2::id(),
            Dex::RaydiumCp => crate::amm_raydium_cp::id(),

            // pmms
            Dex::Humidifi => crate::pmm_humidifi::id(),
            Dex::SolfiV2 => crate::pmm_solfi_v2::id(),
            Dex::Zerofi => crate::pmm_zerofi::id(),
            Dex::ObricV2 => crate::pmm_obric_v2::id(),
        }
    }
}

impl FromStr for Dex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "raydium-cl-v2" | "raydiumclv2" => Ok(Dex::RaydiumClV2),
            "raydium-cp" | "raydiumcp" => Ok(Dex::RaydiumCp),
            "obric-v2" | "obricv2" => Ok(Dex::ObricV2),
            "solfi-v2" | "solfiv2" => Ok(Dex::SolfiV2),
            "zerofi" => Ok(Dex::Zerofi),
            "humidifi" => Ok(Dex::Humidifi),
            _ => Err(format!("unknown dex '{}'", s)),
        }
    }
}

impl From<magnus_router_client::types::Dex> for Dex {
    fn from(value: magnus_router_client::types::Dex) -> Self {
        match value {
            magnus_router_client::types::Dex::RaydiumClV2 => Dex::RaydiumClV2,
            magnus_router_client::types::Dex::RaydiumCp => Dex::RaydiumCp,
            magnus_router_client::types::Dex::ObricV2 => Dex::ObricV2,
            magnus_router_client::types::Dex::SolfiV2 => Dex::SolfiV2,
            magnus_router_client::types::Dex::Zerofi => Dex::Zerofi,
            magnus_router_client::types::Dex::Humidifi => Dex::Humidifi,
        }
    }
}

impl From<Dex> for magnus_router_client::types::Dex {
    fn from(value: Dex) -> Self {
        match value {
            Dex::RaydiumClV2 => magnus_router_client::types::Dex::RaydiumClV2,
            Dex::RaydiumCp => magnus_router_client::types::Dex::RaydiumCp,
            Dex::ObricV2 => magnus_router_client::types::Dex::ObricV2,
            Dex::SolfiV2 => magnus_router_client::types::Dex::SolfiV2,
            Dex::Zerofi => magnus_router_client::types::Dex::Zerofi,
            Dex::Humidifi => magnus_router_client::types::Dex::Humidifi,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct Route {
    pub dexes: Vec<Dex>,
    pub weights: Vec<u8>,
}

impl From<magnus_router_client::types::Route> for Route {
    fn from(value: magnus_router_client::types::Route) -> Self {
        Route { dexes: value.dexes.iter().map(|v| (*v).into()).collect(), weights: value.weights }
    }
}

impl From<Route> for magnus_router_client::types::Route {
    fn from(value: Route) -> Self {
        magnus_router_client::types::Route { dexes: value.dexes.into_iter().map(|v| v.into()).collect(), weights: value.weights }
    }
}
