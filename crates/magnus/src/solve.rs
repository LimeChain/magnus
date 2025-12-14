use std::sync::mpsc::{Receiver, Sender};

use tracing::error;

use crate::{
    Markets, Strategy, StrategyCtx,
    adapters::{QuoteAndSwapResponse, QuoteParams, SwapAndAccountMetas, SwapParams},
};

pub struct SolverCfg {
    pub markets: Markets,
    pub rx: Receiver<DispatchParams>,
    pub tx: Sender<Vec<SwapAndAccountMetas>>,
}

#[derive(Debug)]
pub struct Solver {
    markets: Markets,
    rx: Receiver<DispatchParams>,
    tx: Sender<Vec<SwapAndAccountMetas>>,
}

impl Solver {
    pub fn new(cfg: SolverCfg) -> Self {
        Solver { markets: cfg.markets, rx: cfg.rx, tx: cfg.tx }
    }
}

#[async_trait::async_trait]
impl Strategy for Solver {
    fn name(&self) -> &str {
        "BaseStrategy"
    }

    async fn compute<C: StrategyCtx>(&mut self, _: C) -> eyre::Result<()> {
        while let Ok(params) = self.rx.recv() {
            match params {
                DispatchParams::Quote { params, response_tx } => {
                    // ..
                    let resp = QuoteAndSwapResponse::default();
                    match self.tx.send(vec![SwapAndAccountMetas::default()]) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("failed to send quote response: {}", err);
                            // metrics??
                        }
                    }
                }
                DispatchParams::Swap { params, response_tx } => {
                    // ..
                    let resp = QuoteAndSwapResponse::default();
                    match self.tx.send(vec![SwapAndAccountMetas::default()]) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("failed to send swap response: {}", err);
                            // metrics??
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Dispatch {
    pub rx: Receiver<DispatchParams>,
    pub tx: Sender<DispatchResponse>,
}

#[derive(Debug)]
pub enum DispatchParams {
    Quote { params: QuoteParams, response_tx: oneshot::Sender<DispatchResponse> },
    Swap { params: SwapParams, response_tx: oneshot::Sender<DispatchResponse> },
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum DispatchResponse {
    Quote(QuoteAndSwapResponse),
    Swap(QuoteAndSwapResponse),
}
