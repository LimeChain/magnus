use crate::{ExecuteSignal, TransmitState};

pub trait Strategy {
    fn compute<T: TransmitState, S: ExecuteSignal>(state: T, signal: S) -> eyre::Result<()>;
}

#[derive(Clone, Debug)]
pub enum StrategyKind {
    FCFS,
}

pub struct Solve;

impl Strategy for Solve {
    fn compute<T: TransmitState, S: ExecuteSignal>(state: T, signal: S) -> eyre::Result<()> {
        Ok(())
    }
}
