use tokio::sync::mpsc;

use crate::ExecutionError;

use super::{account::Account, ClientEvent};

#[derive(Debug)]
pub struct SimulatedExchange<Data, Iter, Event>
    where Data: Clone,
          Event: Clone,
          Iter: Iterator<Item = Event> + Clone
{
    pub event_simulated_rx: mpsc::UnboundedReceiver<ClientEvent>,
    pub account: Account<Data, Iter, Event>,
}

impl<Data, Iter, Event> SimulatedExchange<Data, Iter, Event>
    where Data: Clone,
          Event: Clone,
          Iter: Iterator<Item = Event> + Clone
{
    pub fn initiator() -> ExchangeInitiator<Data, Iter, Event>
    {
        ExchangeInitiator::new()
    }

    /// 运行 [`SimulatedExchange`] 并响应各种[`ClientEvent`]。
    pub async fn run(mut self)
    {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                | ClientEvent::FetchOrdersOpen(response_tx, _current_timestamp) => self.account.fetch_orders_open(response_tx).await,
                | ClientEvent::FetchBalances(response_tx, _current_timestamp) => self.account.fetch_balances(response_tx).await,
                | ClientEvent::OpenOrders((open_requests, response_tx), current_timestamp) => {
                    self.account.open_orders(open_requests, response_tx, current_timestamp).await
                }
                | ClientEvent::CancelOrders((cancel_requests, response_tx), current_timestamp) => {
                    self.account.cancel_orders(cancel_requests, response_tx, current_timestamp).await
                }
                | ClientEvent::CancelOrdersAll(response_tx, current_timestamp) => {
                    self.account.cancel_orders_all(response_tx, current_timestamp).await
                }
                | ClientEvent::MarketTrade((instrument, trade), _current_timestamp) => self.account.match_orders(instrument, trade).await,
            }
        }
    }
}

impl<Data, Iter, Event> Default for ExchangeInitiator<Data, Iter, Event>
    where Data: Clone,
          Event: Clone,
          Iter: Iterator<Item = Event> + Clone
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_simulated_rx: Some(rx),
               account: None }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Data, Iter, Event>
    where Data: Clone,
          Event: Clone,
          Iter: Iterator<Item = Event> + Clone
{
    event_simulated_rx: Option<mpsc::UnboundedReceiver<ClientEvent>>,
    account: Option<Account<Data, Iter, Event>>,
}

impl<Data, Iter, Event> ExchangeInitiator<Data, Iter, Event>
    where Data: Clone,
          Event: Clone,
          Iter: Iterator<Item = Event> + Clone
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<ClientEvent>) -> Self
    {
        Self { event_simulated_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Account<Data, Iter, Event>) -> Self
    {
        Self { account: Some(value),
               ..self }
    }

    pub fn initiate(self) -> Result<SimulatedExchange<Data, Iter, Event>, ExecutionError>
    {
        Ok(SimulatedExchange { event_simulated_rx: self.event_simulated_rx
                                                       .ok_or_else(|| ExecutionError::InitiatorIncomplete("event_simulated_rx".to_string()))?,
                               account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))? })
    }
}
