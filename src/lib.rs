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
// pub mod solve;

pub trait TransmitState {}
pub trait PropagateSignal {}

pub trait Strategy {
    fn compute<T: TransmitState, S: PropagateSignal>(state: T, signal: S) -> eyre::Result<()>;
}

pub trait Payload {
    fn execute<T: PropagateSignal>(signal: T) -> eyre::Result<()>;
}
