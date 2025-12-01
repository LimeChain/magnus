//! Magnus is a modular Solana solver; There are a few things to note here:
//! TODO
//! |1| - ..
//! |2| - ..
//! |3| - ..
pub mod adapters;
pub mod bootstrap;
pub mod curves;
pub mod error;
pub mod geyser_client;
pub mod helpers;
pub mod ingest;
pub mod propagate;
pub mod solve;

pub trait TransmitState: Send + Sync {}
pub trait ExecuteSignal: Send + Sync {}
pub trait Strategy: Send + Sync {
    fn compute<T: TransmitState, S: ExecuteSignal>(state: T, signal: S) -> eyre::Result<()>;
}
pub trait Payload: Send + Sync {
    fn execute<T: ExecuteSignal>(signal: T) -> eyre::Result<()>;
}

#[derive(Copy, Clone, Debug)]
pub struct StateTransmitter;
impl TransmitState for StateTransmitter {}

#[derive(Copy, Clone, Debug)]
pub struct SignalExecutor;
impl ExecuteSignal for SignalExecutor {}
