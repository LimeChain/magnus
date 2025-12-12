use std::mem::size_of;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;

pub const MAX_NUM_EVENTS: u16 = 600;
pub const NO_NODE: u16 = u16::MAX;

/// Container for the different EventTypes.
///
/// Events are stored in a fixed-array of nodes. Free nodes are connected by a single-linked list
/// starting at free_head while used nodes form a circular doubly-linked list starting at
/// used_head.
#[derive(Clone)]
pub struct EventHeap {
    pub header: EventHeapHeader,
    pub nodes: [EventNode; MAX_NUM_EVENTS as usize],
    pub reserved: [u8; 64],
}

#[derive(thiserror::Error, Debug)]
pub enum OpenBookError {
    #[error("")]
    SomeError,

    #[error("Name lenght above limit")]
    InvalidInputNameLength,
    #[error("Market cannot be created as expired")]
    InvalidInputMarketExpired,
    #[error("Taker fees should be positive and if maker fees are negative, greater or equal to their abs value")]
    InvalidInputMarketFees,
    #[error("Lots cannot be negative")]
    InvalidInputLots,
    #[error("Lots size above market limits")]
    InvalidInputLotsSize,
    #[error("Input amounts above limits")]
    InvalidInputOrdersAmounts,
    #[error("Price lots should be greater than zero")]
    InvalidInputCancelSize,
    #[error("Expected cancel size should be greater than zero")]
    InvalidInputPriceLots,
    #[error("Peg limit should be greater than zero")]
    InvalidInputPegLimit,
    #[error("The order type is invalid. A taker order must be Market or ImmediateOrCancel")]
    InvalidInputOrderType,
    #[error("Order id cannot be zero")]
    InvalidInputOrderId,
    #[error("Slot above heap limit")]
    InvalidInputHeapSlots,
    #[error("Cannot combine two oracles of different providers")]
    InvalidOracleTypes,
    #[error("Cannot configure secondary oracle without primary")]
    InvalidSecondOracle,

    #[error("This market does not have a `close_market_admin` and thus cannot be closed.")]
    NoCloseMarketAdmin,
    #[error("The signer of this transaction is not this market's `close_market_admin`.")]
    InvalidCloseMarketAdmin,
    #[error("The `open_orders_admin` required by this market to sign all instructions that creates orders is missing or is not valid")]
    InvalidOpenOrdersAdmin,
    #[error("The `consume_events_admin` required by this market to sign all instructions that consume events is missing or is not valid")]
    InvalidConsumeEventsAdmin,
    #[error("Provided `market_vault` is invalid")]
    InvalidMarketVault,

    #[error("Cannot be closed due to the existence of open orders accounts")]
    IndexerActiveOO,

    #[error("Cannot place a peg order due to invalid oracle state")]
    OraclePegInvalidOracleState,
    #[error("oracle type cannot be determined")]
    UnknownOracleType,
    #[error("an oracle does not reach the confidence threshold")]
    OracleConfidence,
    #[error("an oracle is stale")]
    OracleStale,
    #[error("Order id not found on the orderbook")]
    OrderIdNotFound,
    #[error("Event heap contains elements and market can't be closed")]
    EventHeapContainsElements,
    #[error("ImmediateOrCancel is not a PostOrderType")]
    InvalidOrderPostIOC,
    #[error("Market is not a PostOrderType")]
    InvalidOrderPostMarket,
    #[error("would self trade")]
    WouldSelfTrade,
    #[error("The Market has already expired.")]
    MarketHasExpired,
    #[error("Price lots should be greater than zero")]
    InvalidPriceLots,
    #[error("Oracle price above market limits")]
    InvalidOraclePrice,
    #[error("The Market has not expired yet.")]
    MarketHasNotExpired,
    #[error("No correct owner or delegate.")]
    NoOwnerOrDelegate,
    #[error("No correct owner")]
    NoOwner,
    #[error("No free order index in open orders account")]
    OpenOrdersFull,
    #[error("Book contains elements")]
    BookContainsElements,
    #[error("Could not find order in user account")]
    OpenOrdersOrderNotFound,
    #[error("Amount to post above book limits")]
    InvalidPostAmount,
    #[error("Oracle peg orders are not enabled for this market")]
    DisabledOraclePeg,
    #[error("Cannot close a non-empty market")]
    NonEmptyMarket,
    #[error("Cannot close a non-empty open orders account")]
    NonEmptyOpenOrdersPosition,
    #[error("Fill-Or-Kill order would generate a partial execution")]
    WouldExecutePartially,
}

#[derive(Clone)]
pub struct EventHeapHeader {
    free_head: u16,
    used_head: u16,
    count: u16,
    _padd: u16,
    pub seq_num: u64,
}

impl EventHeapHeader {
    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn free_head(&self) -> usize {
        self.free_head as usize
    }

    pub fn used_head(&self) -> usize {
        self.used_head as usize
    }

    fn incr_count(&mut self) {
        self.count += 1;
    }

    fn decr_count(&mut self) {
        self.count -= 1;
    }

    fn incr_event_id(&mut self) {
        self.seq_num += 1;
    }
}

#[derive(Clone, Debug)]
pub struct EventNode {
    next: u16,
    prev: u16,
    _pad: [u8; 4],
    pub event: AnyEvent,
}

impl EventNode {
    pub fn is_free(&self) -> bool {
        self.prev == NO_NODE
    }
}

#[derive(Clone, Debug)]
pub struct AnyEvent {
    pub event_type: u8,
    pub padding: [u8; 143],
}
