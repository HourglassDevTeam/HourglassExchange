use async_trait::async_trait;
use mpsc::UnboundedSender;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AccountEvent,
    ClientExecution, common_skeleton::{
        balance::TokenBalance,
        instrument::Instrument,
        order::{Cancelled, Open, Order},
        trade::Trade,
    }, ExchangeKind, ExecutionError, RequestCancel, RequestOpen,
};

#[derive(Clone, Debug)]
pub struct SimulatedClient
{
    pub local_timestamp: i64,
    pub request_tx: UnboundedSender<SimulatedClientEvent>,
}

#[derive(Debug)]
pub enum SimulatedClientEvent
{
    FetchOrdersOpen(oneshot::Sender<Result<Vec<Order<Open>>, ExecutionError>>, i64),
    FetchBalances(oneshot::Sender<Result<Vec<TokenBalance>, ExecutionError>>, i64),
    OpenOrders((Vec<Order<RequestOpen>>, oneshot::Sender<Vec<Result<Order<Open>, ExecutionError>>>), i64),
    CancelOrders((Vec<Order<RequestCancel>>, oneshot::Sender<Vec<Result<Order<Cancelled>, ExecutionError>>>), i64),
    CancelOrdersAll(oneshot::Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>, i64),
    MarketTrade((Instrument, Trade), i64),
}

#[async_trait]
impl ClientExecution for SimulatedClient
{
    // in our case the 'optional' config parameter in the simulated exchange is an UnboundedSender
    type Config = UnboundedSender<SimulatedClientEvent>;

    // very naturally, the client's kind is determined by and aligned the exchange.
    const CLIENT_KIND: ExchangeKind = ExchangeKind::Simulated;

    async fn init(request_tx: Self::Config, _: UnboundedSender<AccountEvent>, local_timestamp: i64) -> Self
    {
        Self { request_tx, local_timestamp }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        self.request_tx
            .send(SimulatedClientEvent::FetchOrdersOpen(response_tx, self.local_timestamp))
            .expect("[UnilinkExecution] : 模拟交易所目前离线 - 发送获取开放订单FetchOrdersOpen请求失败");
        // 从模拟交易所接收开放订单的响应。
        response_rx.await
                   .expect("[UnilinkExecution] : 模拟交易所目前离线 - 接收获取开放订单 FetchOrdersOpen 响应失败")
    }

    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        self.request_tx
            .send(SimulatedClientEvent::FetchBalances(response_tx, self.local_timestamp))
            .expect("[UnilinkExecution] : 模拟交易所目前离线 - 发送获取账户余额 FetchBalances 请求失败");
        // 从模拟交易所接收账户余额的响应。
        response_rx.await
                   .expect("[UnilinkExecution] : 模拟交易所目前离线 - 接收获取账户余额 FetchBalances 响应失败")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        self.request_tx
            .send(SimulatedClientEvent::OpenOrders((open_requests, response_tx), self.local_timestamp))
            .expect("[UnilinkExecution] : 模拟交易所目前离线 - 发送 OpenOrders 请求失败");
        // 从模拟交易所接收开启订单的响应。
        response_rx.await.expect("[UnilinkExecution] : 模拟交易所目前离线 - 接收 OpenOrders 响应失败")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        self.request_tx
            .send(SimulatedClientEvent::CancelOrders((cancel_requests, response_tx), self.local_timestamp))
            .expect("[UnilinkExecution] : 模拟交易所目前离线 - 发送 CancelOrders 请求失败");
        // 从模拟交易所接收取消订单的响应。
        response_rx.await.expect("[UnilinkExecution] : 模拟交易所目前离线 - 接收 CancelOrders 响应失败")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>
    {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        self.request_tx
            .send(SimulatedClientEvent::CancelOrdersAll(response_tx, self.local_timestamp))
            .expect("[UnilinkExecution] : 模拟交易所目前离线 - 发送 CancelOrdersAll 请求失败");
        // 从模拟交易所接收取消所有订单的响应。
        response_rx.await.expect("[UnilinkExecution] : 模拟交易所目前离线 - 接收 CancelOrdersAll 响应失败")
    }
}
