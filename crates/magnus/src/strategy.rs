use std::sync::mpsc::{Receiver, Sender};

use solana_sdk::pubkey::Pubkey;
use tracing::info;
use utoipa::openapi::info;

use crate::{
    Markets, Strategy, StrategyCtx,
    adapters::{IntQuoteResponse, IntSwapResponse, Quote, QuoteParams, SwapAndAccountMetas, SwapParams, amms::Target},
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

    pub fn whole_swap_route(&self, input_mint: Pubkey, output_mint: Pubkey, amount: u64) -> eyre::Result<Option<(Pubkey, Quote)>> {
        let default_pubkey = Pubkey::default();

        let mut markets = self.markets.lock().unwrap();
        info!("Markets: {:?}", markets);
        let matching_markets: Vec<(Pubkey, Quote)> = markets
            .iter_mut()
            .filter_map(|(market_key, amm)| {
                if *market_key == default_pubkey || !amm.is_active() {
                    info!("Market {} is inactive", market_key);
                    return None;
                }

                let reserve_mints = amm.get_reserve_mints();
                if reserve_mints.contains(&default_pubkey) {
                    info!("Market {} has default pubkey as reserve", market_key);
                    return None;
                }

                let has_input = reserve_mints.contains(&input_mint);
                let has_output = reserve_mints.contains(&output_mint);

                if has_input && has_output {
                    let quote_params = QuoteParams { input_mint, output_mint, amount, swap_mode: crate::adapters::SwapMode::ExactIn };

                    match amm.quote(&quote_params) {
                        Ok(quote) => {
                            info!("Market {} has quote {:?}", market_key, quote);
                            Some((*market_key, quote))
                        }
                        Err(_) => {
                            info!("Market {} failed to quote", market_key);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect();

        let best_market = matching_markets.into_iter().max_by_key(|(_, quote)| quote.out_amount);

        Ok(best_market)
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
                    let quote = match self.whole_swap_route(params.input_mint, params.output_mint, params.amount)? {
                        Some(q) => q.1,
                        None => {
                            info!("no route found");
                            Quote::default()
                        }
                    };

                    if let Ok(()) = response_tx.send(DispatchResponse::Quote(IntQuoteResponse {
                        source: Target::AMMs,
                        input_mint: params.input_mint.to_string(),
                        output_mint: params.output_mint.to_string(),
                        in_amount: params.amount,
                        out_amount: quote.out_amount,
                        ..Default::default()
                    })) {
                        info!("sent from `Strategy` towards `API Server::quote`");
                    };
                }
                // the swap is computed similarly to Quote
                // but the evaluated result is sent downstream towards `Executor`
                // that then proceeds to evaluate the path, attach the relevant accounts,
                // craft the instruction data payload and send the tx/bundles towards
                // an RPC
                DispatchParams::Swap { params, response_tx } => {
                    if self
                        .tx
                        .send(WrappedSwapAndAccountMetas {
                            response_tx,
                            input_mint: params.input_mint,
                            output_mint: params.output_mint,
                            metas: vec![SwapAndAccountMetas::default()],
                        })
                        .is_ok()
                    {
                        info!("sent from Strategy towards `Executor`");
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
