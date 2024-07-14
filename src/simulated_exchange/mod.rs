use tokio::sync::oneshot;

use crate::{
    common_skeleton::{
        balance::TokenBalance,
        instrument::Instrument,
        order::{Cancelled, Open, Order, RequestCancel, RequestOpen},
        trade::Trade,
    },
    error::ExecutionError,
};

pub mod account;
pub mod client;
pub mod instrument_orders;
pub mod simulated_exchange;
mod ws_trade;

#[derive(Debug)]
pub enum SimulatedEvent
{
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>, i64),
    FetchBalances(oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>, i64),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>), i64),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>),
                 i64),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>, i64),
    MarketTrade((Instrument, Trade), i64),
}
