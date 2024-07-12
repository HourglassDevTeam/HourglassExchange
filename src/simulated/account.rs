use crate::{
    error::ExecutionError,
    universal::{
        balance::TokenBalance,
        order::{Cancelled, Opened, Order, OrderKind, RequestCancel, RequestOpen},
    },
};
use serde_json::ser::State;
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub struct AccountInfo<State> {
    config: AccountConfig,
    balances: AccountBalances,
    positions: AccountPositions,
    orders: Vec<Order<State>>,
}

#[derive(Clone, Debug)]
pub struct AccountConfig {
    margin_mode: MarginMode,
    position_mode: PositionMode,
    commission_level: CommissionLevel,
}

#[derive(Clone, Debug)]
pub enum MarginMode {
    SimpleMode,
    SingleCurrencyMargin,
    MultiCurrencyMargin,
    PortfolioMargin,
}

#[derive(Clone, Debug)]
pub enum PositionMode {
    LongShortMode, // Note long/short, only applicable to Futures/Swap
    NetMode,       // Note one side per token per position
}

#[derive(Clone, Debug)]
pub enum CommissionLevel {
    Lv1,
    Lv2,
    // ..........
}

#[derive(Clone, Debug)]
pub struct AccountBalances {
    spot_bal: Vec<SpotBalance>,
}

#[derive(Clone, Debug)]
pub struct SpotBalance {
    currency: String,
    size: f64,
}

#[derive(Clone, Debug)]
pub struct AccountPositions {
    margin_pos: Vec<MarginPosition>,  // useless in backtest
    swap_pos: Vec<SwapPosition>,      // Note useful, and we gonna build it
    futures_pos: Vec<MarginPosition>, // useless
    option_pos: Vec<OptionPosition>,  // useless
}

#[derive(Clone, Debug)]
pub struct MarginPosition {}

#[derive(Clone, Debug)]
pub struct SwapPosition {
    token: String,
    pos_config: SwapPositionConfig,
    pos_size: f64,
    average_price: f64,
    liquidation_price: f64,
    margin: f64,
    pnl: f64,
    fee: f64,
    funding_fee: f64,
}

#[derive(Clone, Debug)]

pub struct SwapPositionConfig {
    pos_margin_mode: PositionMarginMode,
    leverage: f64,
}

#[derive(Clone, Debug)]
pub enum PositionMarginMode {
    Cross,
    Isolated,
}

#[derive(Clone, Debug)]

pub struct FuturesPosition {}

#[derive(Clone, Debug)]

pub struct OptionPosition {}

#[derive(Clone, Debug)]
// NOTE wrap fields with option<> to yield support for initiation in a chained fashion
pub struct AccountBuilder {
    config: Option<AccountConfig>,
    balances: Option<AccountBalances>,
    positions: Option<AccountPositions>,
}

impl AccountBuilder {
    pub fn new() -> Self {
        AccountBuilder {
            config: None,
            balances: None,
            positions: None,
        }
    }
}

impl AccountInfo<State> {
    pub fn initiator() -> AccountBuilder {
        AccountBuilder::new()
    }

    pub fn fetch_orders_open(&self, response_tx: oneshot::Sender<Result<Vec<Order<Opened>>, ExecutionError>>) {
        todo!()
    }

    pub fn fetch_balances(&self, response_tx: oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>) {
        todo!()
    }

    pub fn order_validity_check(kind: OrderKind) -> Result<(), ExecutionError> {
        todo!()
    }

    pub fn try_open_order_atomic(&mut self, request: Order<RequestOpen>) -> Result<Order<Opened>, ExecutionError> {
        Self::order_validity_check(request.state.kind).unwrap();
        todo!()
    }

    pub fn cancel_orders(
        &mut self,
        cancel_requests: Vec<Order<RequestCancel>>,
        response_tx: oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>,
    ) {
        let cancel_results = cancel_requests.into_iter().map(|request| self.try_cancel_order_atomic(request)).collect();
        response_tx.send(cancel_results).unwrap_or_else(|_| {
            // Handle the error if sending fails
        });
    }

    pub fn try_cancel_order_atomic(&mut self, request: Order<RequestCancel>) -> Result<Order<Cancelled>, ExecutionError> {
        todo!()
    }

    pub fn cancel_orders_all(&mut self, response_tx: oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>) {
        todo!()
    }
}
