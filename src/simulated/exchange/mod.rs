// 引入多生产者单消费者通道模块和执行错误类型。
use crate::ExecutionError;
use tokio::sync::mpsc;

// 引入上级模块中的客户账户和模拟事件类型。
use super::{exchange::account::AccountModule, SimulatedCommand};

/// [`SimulatedExchange`] 的账户余额、开放订单、费用和延迟。
pub mod account;

/// 响应 [`SimulatedCommand`] 的 [`SimulatedExchange`]。
#[derive(Debug)]
pub struct SimulatedExchange {
    // 模拟事件的无界接收器。
    pub event_simulated_rx: mpsc::UnboundedReceiver<SimulatedCommand>,
    // 客户账户。
    pub account: AccountModule,
}

impl SimulatedExchange {
    /// 构造一个用于配置新 [`SimulatedExchange`] 的 [`ExchangeInitiator`]。
    pub fn ini() -> ExchangeInitiator {
        ExchangeInitiator::new()
    }

    /// 运行 [`SimulatedExchange`] 并响应 [`SimulatedCommand`]。
    pub async fn run(mut self) {
        // 不断接收并处理模拟事件。
        while let Some(event) = self.event_simulated_rx.recv().await {
            match event {
                // 处理获取开放订单请求。
                | SimulatedCommand::FetchOrdersOpen(response_tx) => self.account.fetch_orders_open(response_tx),
                // 处理获取账户余额请求。
                | SimulatedCommand::FetchBalances(response_tx) => self.account.fetch_balances(response_tx),
                // 处理开启订单请求。
                | SimulatedCommand::OpenOrders((open_requests, response_tx)) => self.account.open_orders(open_requests, response_tx),
                // 处理取消订单请求。
                | SimulatedCommand::CancelOrders((cancel_requests, response_tx)) => self.account.cancel_orders(cancel_requests, response_tx),
                // 处理取消所有订单请求。
                | SimulatedCommand::CancelOrdersAll(response_tx) => self.account.cancel_orders_all(response_tx),
                // 处理市场交易事件。
                | SimulatedCommand::MarketTrade((instrument, trade)) => self.account.match_orders(instrument, trade),
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ExchangeInitiator {
    // 模拟事件的无界接收器，用于构建器。
    event_simulated_rx: Option<mpsc::UnboundedReceiver<SimulatedCommand>>,
    // 客户账户，用于构建器。
    account: Option<AccountModule>,
}

impl ExchangeInitiator {
    fn new() -> Self {
        Self { ..Default::default() }
    }

    // 设置模拟事件的接收器。
    pub fn event_simulated_rx(self, value: mpsc::UnboundedReceiver<SimulatedCommand>) -> Self {
        Self {
            event_simulated_rx: Some(value),
            ..self
        }
    }

    // 设置客户账户。
    pub fn account(self, value: AccountModule) -> Self {
        Self {
            account: Some(value),
            ..self
        }
    }

    pub fn build(self) -> Result<SimulatedExchange, ExecutionError> {
        Ok(SimulatedExchange {
            event_simulated_rx: self
                .event_simulated_rx
                .ok_or_else(|| ExecutionError::BuilderIncomplete("event_simulated_rx".to_string()))?,
            account: self.account.ok_or_else(|| ExecutionError::BuilderIncomplete("account".to_string()))?,
        })
    }
}
