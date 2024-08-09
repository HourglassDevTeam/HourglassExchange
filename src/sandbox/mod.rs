use std::fmt::Debug;

use tokio::sync::mpsc;

use account::Account;

use crate::{error::ExecutionError, sandbox::sandbox_client::SimulatedClientEvent};

pub mod account;
pub mod instrument_orders;
pub mod clickhouse_api;
pub mod sandbox_client;
pub mod utils;
pub mod ws_trade;
pub mod sandbox_orderbook;

#[derive(Debug)]
pub struct SimulatedExchange<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub event_sandbox_rx: mpsc::UnboundedReceiver<SimulatedClientEvent>,
    pub account: Account<Event>,
}

impl<Event> SimulatedExchange<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn initiator() -> ExchangeInitiator<Event>
    {
        ExchangeInitiator::new()
    }

    /// 运行 [`SimulatedExchange`] 并响应各种[`SimulatedClientEvent`]。
    pub async fn run(mut self)
    {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_sandbox_rx.recv().await {
            match event {
                | SimulatedClientEvent::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx).await,
                | SimulatedClientEvent::FetchBalances(response_tx) => self.account.fetch_balances(response_tx).await,
                | SimulatedClientEvent::OpenOrders((open_requests, response_tx)) => self.account.open_requests_into_pendings(open_requests, response_tx).await,
                | SimulatedClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx).await,
                | SimulatedClientEvent::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx).await,
                | SimulatedClientEvent::FetchMarketEvent(market_event) => self.account.match_orders(market_event).await,
            }
        }
    }
}

impl<Event> Default for ExchangeInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_sandbox_rx: Some(rx),
               account: None }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    event_sandbox_rx: Option<mpsc::UnboundedReceiver<SimulatedClientEvent>>,
    account: Option<Account<Event>>,
}

impl<Event> ExchangeInitiator<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_sandbox_rx(self, value: mpsc::UnboundedReceiver<SimulatedClientEvent>) -> Self
    {
        Self { event_sandbox_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Account<Event>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SimulatedExchange<Event>, ExecutionError>
    {
        Ok(SimulatedExchange { event_sandbox_rx: self.event_sandbox_rx
                                                       .ok_or_else(|| ExecutionError::InitiatorIncomplete("event_sandbox_rx".to_string()))?,
                               account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))? })
    }
}
