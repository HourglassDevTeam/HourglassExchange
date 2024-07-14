use tokio::sync::mpsc;

use crate::ExecutionError;

use super::{account::Account, SimulatedEvent};

#[derive(Debug)]
pub struct SimulatedExchange<Data, Event>
where Data: Clone,
      Event: Clone
{
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub account: Account<Data, Event>,
}

impl<Data, Event> SimulatedExchange<Data, Event>
where Data: Clone,
      Event: Clone
{
    pub fn initiator() -> ExchangeInitiator<Data, Event>
    {
        ExchangeInitiator::new()
    }

    /// 运行 [`SimulatedExchange`] 并响应各种[`SimulatedEvent`]。
    pub async fn run(mut self)
    {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                | SimulatedEvent::FetchOrdersOpen(response_tx, _current_timestamp) => self.account.fetch_orders_open(response_tx).await,
                | SimulatedEvent::FetchBalances(response_tx, _current_timestamp) => self.account.fetch_balances(response_tx).await,
                | SimulatedEvent::OpenOrders((open_requests, response_tx), current_timestamp) => {
                    self.account.open_orders(open_requests, response_tx, current_timestamp).await
                }
                | SimulatedEvent::CancelOrders((cancel_requests, response_tx), current_timestamp) => {
                    self.account.cancel_orders(cancel_requests, response_tx, current_timestamp).await
                }
                | SimulatedEvent::CancelOrdersAll(response_tx, current_timestamp) => self.account.cancel_orders_all(response_tx, current_timestamp).await,
                | SimulatedEvent::MarketTrade((instrument, trade), _current_timestamp) => self.account.match_orders(instrument, trade).await,
            }
        }
    }
}

impl<Data, Event> Default for ExchangeInitiator<Data, Event>
where Data: Clone,
      Event: Clone
{
    fn default() -> Self
    {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { event_simulated_rx: Some(rx),
               account: None }
    }
}
#[derive(Debug)]
pub struct ExchangeInitiator<Data, Event>
where Data: Clone,
      Event: Clone
{
    event_simulated_rx: Option<mpsc::UnboundedReceiver<SimulatedEvent>>,
    account: Option<Account<Data, Event>>,
}

impl<Data, Event> ExchangeInitiator<Data, Event>
where Data: Clone,
      Event: Clone
{
    pub fn new() -> Self
    {
        Self { ..Default::default() }
    }

    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<SimulatedEvent>) -> Self
    {
        Self { event_simulated_rx: Some(value),
               ..self }
    }

    pub fn account(self, value: Account<Data, Event>) -> Self
    {
        Self { account: Some(value),
               ..self }
    }

    pub fn initiate(self) -> Result<SimulatedExchange<Data, Event>, ExecutionError>
    {
        Ok(SimulatedExchange { event_simulated_rx: self.event_simulated_rx
                                                       .ok_or_else(|| ExecutionError::InitiatorIncomplete("event_simulated_rx".to_string()))?,
                               account: self.account.ok_or_else(|| ExecutionError::InitiatorIncomplete("account".to_string()))? })
    }
}
