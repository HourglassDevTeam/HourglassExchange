use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::{
    model::order::{Cancelled, Open, Order},
    simulated::SimulatedEvent,
    AccountEvent, ExecutionClient, ExecutionError, ExecutionId, RequestCancel, RequestOpen, SymbolBalance,
};

/// 模拟[`ExecutionClient`]实现
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
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        self.request_tx
            .send(SimulatedEvent::FetchOrdersOpen(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送获取开放订单FetchOrdersOpen请求失败");
        // 从模拟交易所接收开放订单的响应。
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收获取开放订单 FetchOrdersOpen 响应失败")
    }

    async fn fetch_balances(&self) -> Result<Vec<SymbolBalance>, ExecutionError> {
        // 使用 Oneshot 通道与模拟交易所通信
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        self.request_tx
            .send(SimulatedEvent::FetchBalances(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送获取账户余额 FetchBalances 请求失败");
        // 从模拟交易所接收账户余额的响应。
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收获取账户余额 FetchBalances 响应失败")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        self.request_tx
            .send(SimulatedEvent::OpenOrders((open_requests, response_tx)))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 OpenOrders 请求失败");
        // 从模拟交易所接收开启订单的响应。
        response_rx.await.expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 OpenOrders 响应失败")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        self.request_tx
            .send(SimulatedEvent::CancelOrders((cancel_requests, response_tx)))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 CancelOrders 请求失败");
        // 从模拟交易所接收取消订单的响应。
        response_rx.await.expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 CancelOrders 响应失败")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        self.request_tx
            .send(SimulatedEvent::CancelOrdersAll(response_tx))
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 发送 CancelOrdersAll 请求失败");
        // 从模拟交易所接收取消所有订单的响应。
        response_rx
            .await
            .expect("[UniLinkExecution] : 模拟交易所目前离线 - 接收 CancelOrdersAll 响应失败")
    }
}
