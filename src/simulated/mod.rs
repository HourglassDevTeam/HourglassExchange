use tokio::sync::oneshot;

pub mod account;
pub mod client;
pub mod instrument_orders;
pub mod exchange;
mod ws_trade_converter;

#[derive(Debug)]
pub enum SimulatedEvent {
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(oneshot::Sender<Result<Vec<SymbolBalance>, ExecutionError>>),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>)),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>)),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
    MarketTrade((Instrument, PublicTrade)),
}
