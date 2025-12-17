use std::sync::mpsc::{Receiver, Sender};

use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use crate::{
    Markets, Strategy, StrategyCtx,
    adapters::{IntQuoteResponse, IntSwapResponse, QuoteParams, SwapAndAccountMetas, SwapParams, amms::Target},
};

pub struct BaseStrategyCfg {
    pub markets: Markets,
    pub api_server_rx: Receiver<DispatchParams>,
    pub tx: Sender<WrappedSwapAndAccountMetas>,
}

pub struct BaseStrategy {
    pub markets: Markets,
    // the received quote/swap request from the api server
    api_server_rx: Receiver<DispatchParams>,
    // the response we send to the executor if the request we received is swap-related
    // alternatively we immediately respond to the server if the request:
    // - is for quote
    // - fails for one reason or another
    tx: Sender<WrappedSwapAndAccountMetas>,
}

pub struct WrappedSwapAndAccountMetas {
    pub response_tx: oneshot::Sender<DispatchResponse>,
    pub metas: Vec<SwapAndAccountMetas>,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
}

impl BaseStrategy {
    pub fn new(cfg: BaseStrategyCfg) -> Self {
        BaseStrategy { markets: cfg.markets, api_server_rx: cfg.api_server_rx, tx: cfg.tx }
    }
}

#[async_trait::async_trait]
impl Strategy for BaseStrategy {
    async fn compute<C: StrategyCtx>(&mut self, _: C) -> eyre::Result<()> {
        while let Ok(params) = self.api_server_rx.recv() {
            info!("received by `Strategy`");

            match params {
                // since we don't need to submit a transaction
                // the Quote can be evaluated in `Strategy` and directly
                // sent towards the API server
                DispatchParams::Quote { params, response_tx } => {
                    // ..
                    let _resp = IntQuoteResponse::default();

                    match response_tx.send(DispatchResponse::Quote(IntQuoteResponse {
                        source: Target::AMMs,
                        input_mint: params.input_mint.to_string(),
                        output_mint: params.output_mint.to_string(),
                        ..IntQuoteResponse::default()
                    })) {
                        Ok(()) => {
                            info!("sent from `Strategy` towards `API Server::quote`");
                        }
                        Err(_) => {}
                    };
                }
                // the swap is computed similarly to Quote
                // but the evaluated result is sent downstream towards `Executor`
                // that then proceeds to evaluate the path, attach the relevant accounts,
                // craft the instruction data payload and send the tx/bundles towards
                // an RPC
                DispatchParams::Swap { params, response_tx } => {
                    // ..
                    let _resp = IntQuoteResponse::default();
                    match self.tx.send(WrappedSwapAndAccountMetas {
                        response_tx,
                        input_mint: params.input_mint,
                        output_mint: params.output_mint,
                        metas: vec![SwapAndAccountMetas::default()],
                    }) {
                        Ok(_) => {
                            info!("sent from Strategy towards `Executor`");
                        }
                        Err(_) => {}
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum DispatchParams {
    Quote { params: QuoteParams, response_tx: oneshot::Sender<DispatchResponse> },
    Swap { params: SwapParams, response_tx: oneshot::Sender<DispatchResponse> },
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(untagged)]
pub enum DispatchResponse {
    Quote(IntQuoteResponse),
    Swap(IntSwapResponse),
}
