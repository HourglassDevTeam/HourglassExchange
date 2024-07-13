use tokio::sync::oneshot;

use crate::error::ExecutionError;
use crate::universal::balance::TokenBalance;
use crate::universal::instrument::Instrument;
use crate::universal::order::{Cancelled, Open, Order, RequestCancel, RequestOpen};
use crate::universal::trade::Trade;

pub mod account;
mod account_balances;
mod account_config;
mod account_orders;
pub mod client;
pub mod exchange;
pub mod instrument_orders;
mod ws_trade_converter;

#[derive(Debug)]
pub enum SimulatedEvent {
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>)),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
    MarketTrade((Instrument, Trade)),
}
