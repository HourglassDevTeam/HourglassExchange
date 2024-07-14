use tokio::sync::oneshot;

use crate::error::ExecutionError;
use crate::common_skeleton::balance::TokenBalance;
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::order::{Cancelled, Open, Order, RequestCancel, RequestOpen};
use crate::common_skeleton::trade::Trade;

pub mod account;
pub mod client;
pub mod instrument_orders;
pub mod simulated_exchange;
mod ws_trade_converter;

#[derive(Debug)]
pub enum SimulatedEvent {
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>), i64),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>), i64),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>, i64),
    MarketTrade((Instrument, Trade), i64),
}

