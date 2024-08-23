use std::fmt::Debug;

use tokio::sync::mpsc;

use account::Account;

use crate::{error::ExecutionError, sandbox::sandbox_client::SandBoxClientEvent};

pub mod account;
pub mod clickhouse_api;
pub mod instrument_orders;
pub mod sandbox_client;
pub mod sandbox_orderbook;
pub mod utils;
pub mod ws_trade;

#[derive(Debug)]
pub struct SandBoxExchange<Event>
where
    Event: Clone + Send + Sync + Debug + 'static + Ord,
{
    pub event_sandbox_rx: mpsc::UnboundedReceiver<SandBoxClientEvent>,
    pub account: Account<Event>,
}

impl<Event> SandBoxExchange<Event>
where
    Event: Clone + Send + Sync + Debug + 'static + Ord,
{
    pub fn initiator() -> ExchangeInitiator<Event>
    {
        ExchangeInitiator::new()
    }

    /// 运行 [`SandBoxExchange`] 并响应各种[`SandBoxClientEvent`]。
    pub async fn run(mut self)
    {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_sandbox_rx.recv().await {
            match event {
                | SandBoxClientEvent::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx).await,
                | SandBoxClientEvent::FetchBalances(response_tx) => self.account.fetch_balances(response_tx).await,
                | SandBoxClientEvent::OpenOrders((open_requests, response_tx)) => self.account.open_requests_into_pendings(open_requests, response_tx).await,
                | SandBoxClientEvent::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx).await,
                | SandBoxClientEvent::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx).await,
                | SandBoxClientEvent::FetchMarketEvent(market_event) => self.account.match_orders(market_event).await,
            }
        }
    }
}

impl<Event> Default for ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + Debug + 'static + Ord,
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self {
            event_sandbox_rx: Some(rx),
            account: None,
        }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + Debug + 'static + Ord,
{
    event_sandbox_rx: Option<mpsc::UnboundedReceiver<SandBoxClientEvent>>,
    account: Option<Account<Event>>,
}

impl<Event> ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + Debug + 'static + Ord,
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_sandbox_rx(self, value: mpsc::UnboundedReceiver<SandBoxClientEvent>) -> Self
    {
        Self {
            event_sandbox_rx: Some(value),
            ..self
        }
    }

    pub fn account(self, value: Account<Event>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SandBoxExchange<Event>, ExecutionError>
    {
        Ok(SandBoxExchange {
            event_sandbox_rx: self.event_sandbox_rx.ok_or_else(|| ExecutionError::InitiatorIncomplete("event_sandbox_rx".to_string()))?,
            account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))?,
        })
    }
}
