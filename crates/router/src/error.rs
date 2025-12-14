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

    #[msg("Invalid commission rate")]
    InvalidCommissionRate,

    #[msg("Invalid trim rate")]
    InvalidTrimRate,

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

    #[msg("Invalid platform fee rate")]
    InvalidPlatformFeeRate,

    #[msg("Amount out must be greater than 0")]
    AmountOutMustBeGreaterThanZero,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid platform fee amount")]
    InvalidPlatformFeeAmount,

    #[msg("Invalid fee token account")]
    InvalidFeeTokenAccount,

    #[msg("Invalid sa authority")]
    InvalidSaAuthority,

    #[msg("Commission account is none")]
    CommissionAccountIsNone,

    #[msg("Platform fee account is none")]
    PlatformFeeAccountIsNone,

    #[msg("Trim account is none")]
    TrimAccountIsNone,

    #[msg("Invalid fee account")]
    InvalidFeeAccount,

    #[msg("Invalid source token sa")]
    InvalidSourceTokenSa,

    #[msg("Sa authority is none")]
    SaAuthorityIsNone,

    #[msg("Source token sa is none")]
    SourceTokenSaIsNone,

    #[msg("Source token program is none")]
    SourceTokenProgramIsNone,

    #[msg("Destination token sa is none")]
    DestinationTokenSaIsNone,

    #[msg("Destination token program is none")]
    DestinationTokenProgramIsNone,

    #[msg("Invalid trim account")]
    InvalidTrimAccount,

    #[msg("Invalid commission account")]
    InvalidCommissionAccount,

    #[msg("Invalid platform fee account")]
    InvalidPlatformFeeAccount,

    #[msg("Invalid actual amount in")]
    InvalidActualAmountIn,

    #[msg("Unexpected SA token account in CPI")]
    UnexpectedSaTokenAccount,

    #[msg("Invalid source token sa mint")]
    InvalidSourceTokenSaMint,

    #[msg("Invalid destination token sa mint")]
    InvalidDestinationTokenSaMint,
}

#[error_code]
pub enum LimitOrderError {
    #[msg("Invalid account")]
    InvalidAccount,

    #[msg("Invalid trade fee")]
    InvalidTradeFee,

    #[msg("Resolver is exist")]
    ResolverIsExist,

    #[msg("Resolver is not exist")]
    ResolverIsNotExist,

    #[msg("Exceed resolver limit")]
    ExceedResolverLimit,

    #[msg("Invalid deadline")]
    InvalidDeadline,

    #[msg("Invalid making amount")]
    InvalidMakingAmount,

    #[msg("Invalid expect taking amount")]
    InvalidExpectTakingAmount,

    #[msg("Invalid min return amount")]
    InvalidMinReturnAmount,

    #[msg("Actual making amount is zero")]
    ActualMakingAmountIsZero,

    #[msg("Invalid update parameter")]
    InvalidUpdateParameter,

    #[msg("Order expired")]
    OrderExpired,

    #[msg("Order not expired")]
    OrderNotExpired,

    #[msg("Trading paused")]
    TradingPaused,

    #[msg("Only resolver")]
    OnlyResolver,

    #[msg("Not enough trade fee")]
    NotEnoughTradeFee,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid input token owner")]
    InvalidInputTokenOwner,

    #[msg("Invalid output token owner")]
    InvalidOutputTokenOwner,

    #[msg("Input and output token same")]
    InputAndOutputTokenSame,

    #[msg("Invalid fee multiplier")]
    InvalidFeeMultiplier,

    #[msg("Invalid input token account")]
    InvalidInputTokenAccount,
}
