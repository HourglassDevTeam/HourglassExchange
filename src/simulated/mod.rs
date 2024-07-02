use cerebro_data::subscription::trade::PublicTrade;
use cerebro_integration::model::instrument::Instrument;
use tokio::sync::oneshot;

use crate::{Cancelled, ExecutionError, Open, Order, RequestCancel, RequestOpen, TokenBalance};

pub mod exchange;
pub mod execution;

#[derive(Debug)]
pub enum SimulatedEvent {
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>)),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
    MarketTrade((Instrument, PublicTrade)),
}
