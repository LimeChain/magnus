use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Too many hops")]
    TooManyHops,

    #[msg("Min return not reached")]
    MinReturnNotReached,

    #[msg("amount_in must be greater than 0")]
    AmountInMustBeGreaterThanZero,

    #[msg("min_return must be greater than 0")]
    MinReturnMustBeGreaterThanZero,

    #[msg("invalid expect amount out")]
    InvalidExpectAmountOut,

    #[msg("amounts and routes must have the same length")]
    AmountsAndRoutesMustHaveTheSameLength,

    #[msg("total_amounts must be equal to amount_in")]
    TotalAmountsMustBeEqualToAmountIn,

    #[msg("dexes and weights must have the same length")]
    DexesAndWeightsMustHaveTheSameLength,

    #[msg("weights must sum to 100")]
    WeightsMustSumTo100,

    #[msg("Invalid source token account")]
    InvalidSourceTokenAccount,

    #[msg("Invalid destination token account")]
    InvalidDestinationTokenAccount,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid accounts length")]
    InvalidAccountsLength,

    #[msg("Invalid hop accounts")]
    InvalidHopAccounts,

    #[msg("Invalid hop from account")]
    InvalidHopFromAccount,

    #[msg("Swap authority is not signer")]
    SwapAuthorityIsNotSigner,

    #[msg("Invalid authority pda")]
    InvalidAuthorityPda,

    #[msg("Invalid swap authority")]
    InvalidSwapAuthority,

    #[msg("Invalid program id")]
    InvalidProgramId,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Calculation error")]
    CalculationError,

    #[msg("Invalid accounts and instruction length")]
    InvalidBundleInput,

    #[msg("Amount out must be greater than 0")]
    AmountOutMustBeGreaterThanZero,

    #[msg("Invalid sa authority")]
    InvalidSaAuthority,

    #[msg("Invalid actual amount in")]
    InvalidActualAmountIn,

    #[msg("Unexpected SA token account in CPI")]
    UnexpectedSaTokenAccount,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid Goonfi parameters")]
    InvalidGoonfiParameters,
}
