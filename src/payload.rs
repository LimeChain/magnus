use std::sync::mpsc;

use solana_client::nonblocking::rpc_client::RpcClient;

use crate::{ExecuteSignal, adapters::SwapAndAccountMetas, solve::DispatchResponse};

#[async_trait::async_trait]
pub trait Executor: Send {
    async fn execute<T: ExecuteSignal>(&mut self, signal: T) -> eyre::Result<()>;
}

pub struct BaseExecutorCfg {
    pub client: std::sync::Arc<RpcClient>,
    pub solver_rx: mpsc::Receiver<Vec<SwapAndAccountMetas>>,
    pub executor_tx: mpsc::Sender<DispatchResponse>,
}

pub struct BaseExecutor {
    client: std::sync::Arc<RpcClient>,
    // receives swaps & accounts from the solver
    solver_rx: mpsc::Receiver<Vec<SwapAndAccountMetas>>,
    // transmits the response downstream towards the API server
    executor_tx: mpsc::Sender<DispatchResponse>,
}

impl BaseExecutor {
    pub fn new(cfg: BaseExecutorCfg) -> Self {
        BaseExecutor { client: cfg.client, solver_rx: cfg.solver_rx, executor_tx: cfg.executor_tx }
    }
}

#[async_trait::async_trait]
impl Executor for BaseExecutor {
    async fn execute<T: ExecuteSignal>(&mut self, signal: T) -> eyre::Result<()> {
        // ..

        while let Ok(swaps) = self.solver_rx.recv() {
            // ..
        }

        Ok(())
    }
}
