use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AccountEvent,
    ExecutionClient,
    ExecutionError, ExecutionId, model::order::{Cancelled, Open, Order}, RequestCancel, RequestOpen, simulated::SimulatedEvent,
    SymbolBalance,
};

/// 模拟[`ExecutionClient`]实现，可用于 Cerebro。
/// [`SimulatedExchange`](super::exchange::SimulatedExchange).
#[derive(Clone, Debug)]
pub struct SimulatedExecution {
    pub request_tx: mpsc::UnboundedSender<SimulatedEvent>,
}

#[async_trait]
impl ExecutionClient for SimulatedExecution {
    const CLIENT: ExecutionId = ExecutionId::Simulated;
    type Config = mpsc::UnboundedSender<SimulatedEvent>;

    async fn init(request_tx: Self::Config, _: mpsc::UnboundedSender<AccountEvent>) -> Self {
        Self { request_tx }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        // 使用 Oneshot 通道与模拟交易所通信
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();

        // Send FetchOrdersOpen request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::FetchOrdersOpen(response_tx))
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to send FetchOrdersOpen request");

        // Receive FetchOrdersOpen response from the SimulatedExchange
        response_rx
            .await
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to receive FetchOrdersOpen response")
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        // 使用 Oneshot 通道与模拟交易所通信
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();

        // Send FetchBalances request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::FetchBalances(response_tx))
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to send FetchBalances request");

        // Receive FetchBalances response from the SimulatedExchange
        response_rx
            .await
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to receive FetchBalances response")
    }

    async fn open_orders(
        &self,
        open_requests: Vec<Order<RequestOpen>>,
    ) -> Vec<Result<Order<Open>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        // Send OpenOrders request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::OpenOrders((open_requests, response_tx)))
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to send OpenOrders request");
        // 从模拟交易所接收开启订单的响应。
        // Receive OpenOrders response from the SimulatedExchange
        response_rx
            .await
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to receive OpenOrders response")
    }

    async fn cancel_orders(
        &self,
        cancel_requests: Vec<Order<RequestCancel>>,
    ) -> Vec<Result<Order<Cancelled>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        // Send CancelOrders request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::CancelOrders((cancel_requests, response_tx)))
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to send CancelOrders request");
        // 从模拟交易所接收取消订单的响应。
        // Receive CancelOrders response from the SimulatedExchange
        response_rx
            .await
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to receive CancelOrders response")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        // Send CancelOrdersAll request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::CancelOrdersAll(response_tx))
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to send CancelOrdersAll request");
        // 从模拟交易所接收取消所有订单的响应。
        // Receive CancelOrdersAll response from the SimulatedExchange
        response_rx
            .await
            .expect("[CerebroBroker] : SimulatedExchange is offline - failed to receive CancelOrdersAll response")
    }
}
