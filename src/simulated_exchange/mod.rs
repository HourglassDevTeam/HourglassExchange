use tokio::sync::mpsc;

use account::Account;

use crate::{error::ExecutionError, simulated_exchange::simulated_client::SimulatedClientEvent};

pub mod account;
mod data_from_clickhouse;
pub mod instrument_orders;
pub mod load_from_clickhouse;
pub mod simulated_client;
pub mod utils;
pub mod ws_trade;

#[derive(Debug)]
pub struct SimulatedExchange<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedClientEvent>,
    pub account: Account<Event>,
}

impl<Event> SimulatedExchange<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub fn initiator() -> ExchangeInitiator<Event>
    {
        ExchangeInitiator::new()
    }

    /// 运行 [`SimulatedExchange`] 并响应各种[`SimulatedClientEvent`]。
    pub async fn run(mut self)
    {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                | SimulatedClientEvent::FetchOrdersOpen(response_tx, _current_timestamp) => self.account.fetch_orders_open(response_tx).await,
                | SimulatedClientEvent::FetchBalances(response_tx, _current_timestamp) => self.account.fetch_balances(response_tx).await,
                | SimulatedClientEvent::OpenOrders((open_requests, response_tx), current_timestamp) => {
                    self.account.open_orders(open_requests, response_tx, current_timestamp).await
                }
                | SimulatedClientEvent::CancelOrders((cancel_requests, response_tx), current_timestamp) => {
                    self.account.cancel_orders(cancel_requests, response_tx, current_timestamp).await
                }
                | SimulatedClientEvent::CancelOrdersAll(response_tx, current_timestamp) => self.account.cancel_orders_all(response_tx, current_timestamp).await,
                | SimulatedClientEvent::MarketTrade((instrument, trade), _current_timestamp) => self.account.match_orders(instrument, trade).await,
            }
        }
    }
}

impl<Event> Default for ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self {
            event_simulated_rx: Some(rx),
            account: None,
        }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    event_simulated_rx: Option<mpsc::UnboundedReceiver<SimulatedClientEvent>>,
    account: Option<Account<Event>>,
}

impl<Event> ExchangeInitiator<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<SimulatedClientEvent>) -> Self
    {
        Self {
            event_simulated_rx: Some(value),
            ..self
        }
    }

    pub fn account(self, value: Account<Event>) -> Self
    {
        Self { account: Some(value), ..self }
    }

    pub fn initiate(self) -> Result<SimulatedExchange<Event>, ExecutionError>
    {
        Ok(SimulatedExchange {
            event_simulated_rx: self.event_simulated_rx
                .ok_or_else(|| ExecutionError::InitiatorIncomplete("event_simulated_rx".to_string()))?,
            account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))?,
        })
    }
}
