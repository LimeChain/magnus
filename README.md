# Magnus

Magnus is a modular Solana solver aimed at unifying the components required to integrate with any of the liquidity sources deployed on Solana. This includes AMMs, Prop AMMs, and Aggregators.

The system is made up of two core components:

- [magnus](./crates/magnus/src/lib.rs) - the offchain service that aggregates account state in real-time, computes predefined strategies and sends tailored transactions (or bundles).
- [router](./crates/router/src/lib.rs) - the onchain router that facilitates the swaps.

The current implementation is discrete enough to work both as a standalone, runnable binary, and a plug-and-play framework.

The currently implemented liquidity sources include:

- Humidifi
- ObricV2
- Raydium (Constant Product)
- Raydium (Concentrated Liquidity)

Since some (most) Prop AMMs are deliberately obfuscated black-boxes, instead of locally storing properly deserialised state and computing swap amounts, we're directly simulating through a built-in chroot-like shell. All adapters require an implementation of (deliberately) similar-to-jupiter interface that abstracts away any of the exchange-specific logic. Thanks to the interface, the liquidity sources are treated interchangeably.

```rs
pub trait Amm: Adapter + Send + Sync + Debug {
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> eyre::Result<Self> where Self: Sized;
    fn label(&self) -> String;
    fn program_id(&self) -> Pubkey;
    fn key(&self) -> Pubkey;
    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync>;
    fn get_reserve_mints(&self) -> Vec<Pubkey>;
    fn get_accounts_to_update(&self) -> Vec<Pubkey>;
    fn update(&mut self, account_map: &AccountMap, slot: Option<u64>) -> eyre::Result<()>;
    fn quote(&self, quote_params: &QuoteParams) -> eyre::Result<Quote>;
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> eyre::Result<SwapAndAccountMetas>;
    fn has_dynamic_accounts(&self) -> bool { false }
    fn requires_update_for_reserve_mints(&self) -> bool { false }
    fn supports_exact_out(&self) -> bool { false }
    fn unidirectional(&self) -> bool { false }
    fn get_accounts_len(&self) -> usize { 32 }
    fn underlying_liquidities(&self) -> Option<HashSet<Pubkey>> { None }
    fn is_active(&self) -> bool { true }
}
```

Each core solver component of the offchain subsystem is an implementation of one of the following traits:

````rs
/// Ingest is responsible for collecting and processing raw data from external sources.
///
/// Implementors of this trait define how data is ingested into the system, whether from
/// Geyser streams, RPC endpoints, websockets, or other data sources. The ingest phase
/// typically populates shared state (like `Markets` or `AccountMap`) that downstream
/// strategies consume.
///
/// # Examples
///
/// ```ignore
/// struct GeyserIngestor { /* ... */ }
///
/// #[async_trait::async_trait]
/// impl Ingest for GeyserIngestor {
///     async fn ingest<C: IngestCtx>(&mut self, ctx: C) -> eyre::Result<()> {
///         // Subscribe to account updates, parse market data, etc.
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Ingest: Send {
    fn name(&self) -> &str {
        "BaseIngest"
    }

    async fn ingest<C: IngestCtx>(&mut self, ctx: C) -> eyre::Result<()>;
}

/// Strategy defines the core logic for identifying and evaluating swap opportunities.
///
/// Implementors analyze ingested market state, compute arbitrage opportunities, optimal
/// routes, or other trading signals. Strategies consume shared state (typically `Markets`)
/// and produce actionable signals that executors can act upon.
///
/// The strategy layer is decoupled from both data ingestion and execution, allowing
/// different strategies to be composed and tested independently.
///
/// # Examples
///
/// ```ignore
/// struct ArbStrategy { /* ... */ }
///
/// #[async_trait::async_trait]
/// impl Strategy for ArbStrategy {
///     async fn compute<C: StrategyCtx>(&mut self, ctx: C) -> eyre::Result<()> {
///         // Analyze opportunities, send signals
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Strategy: Send {
    fn name(&self) -> &str {
        "BaseStrategy"
    }

    async fn compute<C: StrategyCtx>(&mut self, ctx: C) -> eyre::Result<()>;
}

/// Executor handles the submission and management of transactions onchain.
///
/// Implementors receive trading signals from strategies and are responsible for:
/// - Building and signing transactions
/// - Managing transaction submission and retry logic
/// - Handling MEV protection (e.g., Jito bundles)
/// - Monitoring execution results and updating state
///
/// The executor is the final stage in the solver pipeline and is the only component
/// that's allowed to submit txs.
///
/// # Examples
///
/// ```ignore
/// struct JitoExecutor { /* ... */ }
///
/// #[async_trait::async_trait]
/// impl Executor for JitoExecutor {
///     async fn execute<C: ExecutorCtx>(&mut self, signal: C) -> eyre::Result<()> {
///         // Build tx, submit bundle, monitor execution
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Executor: Send {
    fn name(&self) -> &str {
        "BaseExecutor"
    }

    async fn execute<C: ExecutorCtx>(&mut self, signal: C) -> eyre::Result<()>;
}
````

The solver exposes the following endpoints one can trigger a quote or swap with:

```
=> `/api/v1/quote`
=> `/api/v1/swap`
```

The API Server's endpoints documentation can be found, once the solver's been span-up, at `0:0:0:0:19000`.
