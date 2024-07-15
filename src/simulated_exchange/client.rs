
use async_trait::async_trait;
use mpsc::UnboundedSender;
use tokio::sync::{mpsc, oneshot};

use crate::{common_skeleton::order::{Cancelled, Open, Order}, simulated_exchange::SimulatedEvent, AccountEvent, ExecutionError, RequestCancel, RequestOpen, ExchangeKind, ClientExecution};
use crate::common_skeleton::balance::TokenBalance;

/// 模拟[`ClientExecution`]实现，可用于 Cerebro。
/// [`SimulatedExchange`](super::exchange::SimulatedExchange).
#[derive(Clone, Debug)]
pub struct SimulatedClient {
    pub local_timestamp: i64,
    pub request_tx: UnboundedSender<SimulatedEvent>,
}

#[async_trait]
impl ClientExecution for SimulatedClient {
    // very naturally, the client's kind is determined by the exchange.
    const CLIENT_KIND = ExchangeKind::Simulated;
    // in our case the 'optional' config parameter in the simulated exchange is an UnboundedSender
    type Config = UnboundedSender<SimulatedEvent>;

    async fn init(request_tx: Self::Config, _: UnboundedSender<AccountEvent>,local_timestamp:i64) -> Self {
        Self { request_tx, local_timestamp}
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError> {
        // 使用 Oneshot 通道与模拟交易所通信
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        // Send FetchOrdersOpen request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::FetchOrdersOpen(response_tx,self.local_timestamp))
            .expect("[TideBroker] : 模拟交易所目前离线 - 发送获取开放订单FetchOrdersOpen请求失败");
        // 从模拟交易所接收开放订单的响应。
        // Receive FetchOrdersOpen response from the SimulatedExchange
        response_rx
            .await
            .expect("[TideBroker] : 模拟交易所目前离线 - 接收获取开放订单 FetchOrdersOpen 响应失败")
    }

    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError> {
        // 使用 Oneshot 通道与模拟交易所通信
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        // Send FetchBalances request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::FetchBalances(response_tx, self.local_timestamp))
            .expect("[TideBroker] : 模拟交易所目前离线 - 发送获取账户余额 FetchBalances 请求失败");
        // 从模拟交易所接收账户余额的响应。
        // Receive FetchBalances response from the SimulatedExchange
        response_rx
            .await
            .expect("[TideBroker] : 模拟交易所目前离线 - 接收获取账户余额 FetchBalances 响应失败")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        // Send OpenOrders request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::OpenOrders((open_requests, response_tx),self.local_timestamp))
            .expect("[TideBroker] : 模拟交易所目前离线 - 发送 OpenOrders 请求失败");
        // 从模拟交易所接收开启订单的响应。
        // Receive OpenOrders response from the SimulatedExchange
        response_rx.await.expect("[TideBroker] : 模拟交易所目前离线 - 接收 OpenOrders 响应失败")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        // Send CancelOrders request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::CancelOrders((cancel_requests, response_tx),self.local_timestamp))
            .expect("[TideBroker] : 模拟交易所目前离线 - 发送 CancelOrders 请求失败");
        // 从模拟交易所接收取消订单的响应。
        // Receive CancelOrders response from the SimulatedExchange
        response_rx.await.expect("[TideBroker] : 模拟交易所目前离线 - 接收 CancelOrders 响应失败")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError> {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        // Oneshot channel to communicate with the SimulatedExchange
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        // Send CancelOrdersAll request to the SimulatedExchange
        self.request_tx
            .send(SimulatedEvent::CancelOrdersAll(response_tx, self.local_timestamp))
            .expect("[TideBroker] : 模拟交易所目前离线 - 发送 CancelOrdersAll 请求失败");
        // 从模拟交易所接收取消所有订单的响应。
        // Receive CancelOrdersAll response from the SimulatedExchange
        response_rx
            .await
            .expect("[TideBroker] : 模拟交易所目前离线 - 接收 CancelOrdersAll 响应失败")
    }
}
