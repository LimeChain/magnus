//! Holds some of the constants shared across the router and the offchain service(s)
//!
//! MMs:
//! - the program id is used to indicate the route
//! - accounts_len - the number of accounts required (and expected) for successful swap at a particular exchange
//! - args_len - the number of bytes expected as instruction data

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

    declare_id!("SV2EYYJyRz2YhfXwXnhNAevDEui5Q6yrfyo13WtupPF");
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

pub mod spl_token {
    use anchor_lang::prelude::*;

    declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
}
