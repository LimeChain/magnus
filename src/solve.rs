use crate::{ExecuteSignal, TransmitState};

#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    async fn compute<T: TransmitState, S: ExecuteSignal>(state: T, signal: S) -> eyre::Result<()>;
}

#[derive(Clone, Debug)]
pub enum StrategyKind {
    FCFS,
}

#[derive(Clone, Debug)]
pub struct Solver;

impl Solver {
    pub fn new() -> Self {
        Solver {}
    }
}

#[async_trait::async_trait]
impl Strategy for Solver {
    async fn compute<T: TransmitState, S: ExecuteSignal>(state: T, signal: S) -> eyre::Result<()> {
        unimplemented!()
    }
}
