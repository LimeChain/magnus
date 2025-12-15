//! Magnus is a modular Solana solver; There are a few things to note here:
//! TODO
//! |1| - ..
//! |2| - ..
//! |3| - ..

pub mod adapters;
pub mod api_server;
pub mod bootstrap;
pub mod curves;
pub mod error;
pub mod executor;
pub mod geyser_client;
pub mod helpers;
pub mod ingest;
#[cfg(feature = "metrics")]
pub mod metrics_server;
pub mod strategy;

/// HashMap<Pubkey, Vec<Pubkey>>
///
///   -> the key is the program (amm) addr
///   -> the value is a list of the markets we collect data for
pub type Programs = std::collections::HashMap<solana_sdk::pubkey::Pubkey, Vec<solana_sdk::pubkey::Pubkey>>;

/// Arc<Mutex<HashMap<Pubkey, Box<dyn Amm>>>>
///
///   -> the pubkey is the market addr
///   -> the value is the actual market impl
pub type Markets = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<solana_sdk::pubkey::Pubkey, Box<dyn crate::adapters::amms::Amm>>>>;

/// HashMap<Pubkey, Pubkey>
///
///   -> the key is an account addr we receive subscription updates for
///   -> the value is the market addr (i.e the 'owner acc' of the key account addr)
pub type StateAccountToMarket = std::collections::HashMap<solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey>;

/// HashMap<Pubkey, Account> (aka AccountMap)
///   -> the key is the account that we follow for updates
///   -> the value is the actual account structure
pub type AccountMap = std::collections::HashMap<solana_sdk::pubkey::Pubkey, solana_sdk::account::Account, ahash::RandomState>;

/// Trait-type of context, expected by and passable towards [`Ingest::ingest`].
pub trait IngestCtx: Send + Sync {}

/// Trait-type of context, expected by and passable towards [`Strategy::compute`].
pub trait StrategyCtx: Send + Sync {}

/// Trait-type of context, expected by and passable towards [`Executor::execute`].
pub trait ExecutorCtx: Send + Sync {}

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

/// Bare-bones context
#[derive(Copy, Clone)]
pub struct EmptyCtx;

impl IngestCtx for EmptyCtx {}
impl StrategyCtx for EmptyCtx {}
impl ExecutorCtx for EmptyCtx {}
