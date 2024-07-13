use super::{account::Account, SimulatedEvent};
use crate::ExecutionError;
use tokio::sync::mpsc;

/// 响应 [`SimulatedEvent`] 的 [`SimulatedExchange`]。
#[derive(Debug)]
pub struct SimulatedExchange<Data, Event> {
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
    pub account: Account<Data, Event>,
}

impl <Data, Event>SimulatedExchange<Data, Event> {
    pub fn builder() -> ExchangeBuilder<Data, Event> {
        ExchangeBuilder::new()
    }
    /// 运行 [`SimulatedExchange`] 并响应各种[`SimulatedEvent`]。
    pub async fn run(mut self) {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                SimulatedEvent::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx),
                SimulatedEvent::FetchBalances(response_tx) => self.account.fetch_balances(response_tx),
                SimulatedEvent::OpenOrders((open_requests, response_tx)) => self.account.open_orders(open_requests, response_tx),
                SimulatedEvent::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx),
                SimulatedEvent::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx),
                SimulatedEvent::MarketTrade((instrument, trade)) => self.account.match_orders(instrument, trade),
            }
        }
    }
}


impl<Data, Event> Default for ExchangeBuilder<Data, Event> {
    fn default() -> Self {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self {
            event_simulated_rx: Some(rx),
            account: None,
        }
    }
}
#[derive(Debug)]
pub struct ExchangeBuilder<Data, Event> {
    event_simulated_rx: Option<mpsc::UnboundedReceiver<SimulatedEvent>>,
    // 客户账户，用于构建器。
    account: Option<Account<Data, Event>>,
}

impl <Data, Event>ExchangeBuilder<Data, Event> {
    fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<SimulatedEvent>) -> Self {
        Self {
            event_simulated_rx: Some(value),
            ..self
        }
    }

    pub fn account(self, value: Account<Data, Event>) -> Self {
        Self {
            account: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<SimulatedExchange<Data, Event>, ExecutionError> {
        Ok(SimulatedExchange {
            event_simulated_rx: self
                .event_simulated_rx
                .ok_or_else(|| ExecutionError::BuilderIncomplete("event_simulated_rx".to_string()))?,
            account: self.account.ok_or_else(|| ExecutionError::BuilderIncomplete("account".to_string()))?,
        })
    }
}
