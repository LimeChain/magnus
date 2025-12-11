// fuck my life - https://github.com/openbook-dex/openbook-v2/blob/f3e17421e675b083b584867594bf3cf4f675d156/lib/client/src/jup.rs

use std::cell::RefCell;

use anchor_lang::{__private::bytemuck::Zeroable, prelude::*};
use anchor_spl::token::Token;
use eyre::Result;
use fixed::types::I80F48;
use openbook_v2::{
    accounts::PlaceTakeOrder,
    accounts_zerocopy,
    pubkey_option::NonZeroPubkeyOption,
    state::{BookSide, EventHeap, Market, Orderbook, Side},
};
/// An abstraction in order to share reserve mints and necessary data
use solana_sdk::{pubkey::Pubkey, sysvar::clock};

use crate::{
    adapters::{
        Adapter, Swap,
        amms::{
            AccountMap, Amm, AmmContext, KeyedAccount, OPENBOOK_V2, Quote, QuoteParams, Side, SwapAndAccountMetas, SwapParams,
            obric_v2::state::{PriceFeed, SSTradingPair},
            openbook_v2::state::EventHeap,
        },
    },
    book::{Amounts, amounts_from_book},
    remaining_accounts_to_crank,
    util::ZeroCopyDeserialize,
};

#[derive(Clone, Debug)]
pub struct OpenBookMarket {
    market: Market,
    event_heap: EventHeap,
    bids: BookSide,
    asks: BookSide,
    timestamp: u64,
    key: Pubkey,
    label: String,
    related_accounts: Vec<Pubkey>,
    reserve_mints: [Pubkey; 2],
    oracle_price: Option<I80F48>,
    is_permissioned: bool,
}

impl Adapter for OpenBookMarket {}

impl Amm for OpenBookMarket {
    fn label(&self) -> String {
        self.label.clone()
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn program_id(&self) -> Pubkey {
        OPENBOOK_V2
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.reserve_mints.to_vec()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        self.related_accounts.to_vec()
    }

    fn from_keyed_account(keyed_account: &KeyedAccount, _: &AmmContext) -> Result<Self> {
        let market = Market::try_deserialize_from_slice(&mut keyed_account.account.data.as_slice())?;

        let is_permissioned = market.open_orders_admin.is_some();
        let related_accounts = if is_permissioned {
            vec![]
        } else {
            let mut accs = vec![market.bids, market.asks, market.event_heap, clock::ID];

            accs.extend([market.oracle_a, market.oracle_b].into_iter().filter_map(Option::<Pubkey>::from));
            accs
        };

        Ok(OpenBookMarket {
            market,
            key: keyed_account.key,
            label: market.name().to_string(),
            related_accounts,
            reserve_mints: [market.base_mint, market.quote_mint],
            event_heap: EventHeap::zeroed(),
            bids: BookSide::zeroed(),
            asks: BookSide::zeroed(),
            oracle_price: None,
            timestamp: 0,
            is_permissioned,
        })
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        if self.is_permissioned {
            return Ok(());
        }

        let bids_data = account_map.get(&self.market.bids).unwrap();
        self.bids = BookSide::try_deserialize_from_slice(&mut bids_data.data.as_slice()).unwrap();

        let asks_data = account_map.get(&self.market.asks).unwrap();
        self.asks = BookSide::try_deserialize_from_slice(&mut asks_data.data.as_slice()).unwrap();

        let event_heap_data = account_map.get(&self.market.event_heap).unwrap();
        self.event_heap = EventHeap::try_deserialize_from_slice(&mut event_heap_data.data.as_slice()).unwrap();

        let clock_data = account_map.get(&clock::ID).unwrap();
        let clock: Clock = bincode::deserialize(clock_data.data.as_slice())?;

        let oracle_acc = |nonzero_pubkey: NonZeroPubkeyOption| -> Option<accounts_zerocopy::KeyedAccount> {
            let key = Option::from(nonzero_pubkey)?;
            let account = account_map.get(&key).unwrap().clone();
            Some(accounts_zerocopy::KeyedAccount { key, account })
        };

        self.oracle_price = self.market.oracle_price(oracle_acc(self.market.oracle_a).as_ref(), oracle_acc(self.market.oracle_b).as_ref(), clock.slot)?;

        self.timestamp = clock.unix_timestamp.try_into().unwrap();

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        if self.is_permissioned {
            return Ok(Quote { ..Quote::default() });
        }

        let side = if quote_params.input_mint == self.market.quote_mint { Side::Bid } else { Side::Ask };

        let input_amount = i64::try_from(quote_params.amount)?;

        // quote params can have exact in (which is implemented here) and exact out which is not implemented
        // check with jupiter to add to their API exact_out support
        let (max_base_lots, max_quote_lots_including_fees) = match side {
            Side::Bid => (self.market.max_base_lots(), input_amount + (self.market.quote_lot_size - 1) / self.market.quote_lot_size),
            Side::Ask => (input_amount + (self.market.base_lot_size - 1) / self.market.base_lot_size, self.market.max_quote_lots()),
        };

        let bids_ref = RefCell::new(self.bids);
        let asks_ref = RefCell::new(self.asks);
        let book = Orderbook { bids: bids_ref.borrow_mut(), asks: asks_ref.borrow_mut() };

        let order_amounts: Amounts = amounts_from_book(book, side, max_base_lots, max_quote_lots_including_fees, &self.market, self.oracle_price, 0)?;

        let (in_amount, out_amount) = match side {
            Side::Bid => (order_amounts.total_quote_taken_native - order_amounts.fee, order_amounts.total_base_taken_native),
            Side::Ask => (order_amounts.total_base_taken_native, order_amounts.total_quote_taken_native + order_amounts.fee),
        };

        Ok(Quote { in_amount, out_amount, fee_mint: self.market.quote_mint, fee_amount: order_amounts.fee, ..Quote::default() })
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let SwapParams { source_mint, user_destination_token_account, user_source_token_account, user_transfer_authority, .. } = swap_params;

        let source_is_quote = source_mint == &self.market.quote_mint;

        let (side, jup_side) = if source_is_quote { (Side::Bid, Side::Bid) } else { (Side::Ask, Side::Ask) };

        if self.is_permissioned {
            Ok(SwapAndAccountMetas { swap: Swap::OpenbookV2 { side: jup_side }, account_metas: vec![] })
        } else {
            let (user_quote_account, user_base_account) = if source_is_quote {
                (*user_source_token_account, *user_destination_token_account)
            } else {
                (*user_destination_token_account, *user_source_token_account)
            };

            let accounts = PlaceTakeOrder {
                signer: *user_transfer_authority,
                penalty_payer: *user_transfer_authority,
                market: self.key,
                market_authority: self.market.market_authority,
                bids: self.market.bids,
                asks: self.market.asks,
                user_base_account,
                user_quote_account,
                market_base_vault: self.market.market_base_vault,
                market_quote_vault: self.market.market_quote_vault,
                event_heap: self.market.event_heap,
                oracle_a: Option::from(self.market.oracle_a),
                oracle_b: Option::from(self.market.oracle_b),
                token_program: Token::id(),
                system_program: System::id(),
                open_orders_admin: None,
            };

            let mut account_metas = accounts.to_account_metas(None);

            let bids_ref = RefCell::new(self.bids);
            let asks_ref = RefCell::new(self.asks);
            let book = Orderbook { bids: bids_ref.borrow_mut(), asks: asks_ref.borrow_mut() };

            let remaining_accounts = remaining_accounts_to_crank(book, side, &self.market, self.oracle_price, self.timestamp)?;

            let remaining_accounts: Vec<AccountMeta> = remaining_accounts.iter().map(|&pubkey| AccountMeta::new(pubkey, false)).collect();
            account_metas.extend(remaining_accounts);

            Ok(SwapAndAccountMetas { swap: Swap::OpenbookV2 { side: { jup_side } }, account_metas })
        }
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }
}
