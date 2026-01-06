use std::sync::mpsc;

use solana_client::nonblocking::rpc_client::RpcClient;
use tracing::info;

use crate::{
    Executor, ExecutorCtx,
    adapters::{IntSwapResponse, amms::Target},
    strategy::{DispatchResponse, WrappedSwapAndAccountMetas},
};

pub struct BaseExecutorCfg {
    pub client: std::sync::Arc<RpcClient>,
    pub solver_rx: mpsc::Receiver<WrappedSwapAndAccountMetas>,
}

pub struct BaseExecutor {
    _client: std::sync::Arc<RpcClient>,
    // receives swaps & accounts from the solver
    solver_rx: mpsc::Receiver<WrappedSwapAndAccountMetas>,
}

impl BaseExecutor {
    pub fn new(cfg: BaseExecutorCfg) -> Self {
        BaseExecutor { _client: cfg.client, solver_rx: cfg.solver_rx }
    }
}

#[async_trait::async_trait]
impl Executor for BaseExecutor {
    async fn execute<C: ExecutorCtx>(&mut self, _: C) -> eyre::Result<()> {
        // ..

        while let Ok(swaps) = self.solver_rx.recv() {
            info!("received by `Executor`");
            //tokio::time::sleep(Duration::from_secs(2)).await;
            if let Ok(()) = swaps.response_tx.send(DispatchResponse::Swap(IntSwapResponse {
                source: Target::AMMs,
                input_mint: swaps.input_mint.to_string(),
                output_mint: swaps.output_mint.to_string(),
                ..IntSwapResponse::default()
            })) {
                info!("sent from `Executor` towards `API Server::swap`")
            }
        }

        Ok(())
    }
}
