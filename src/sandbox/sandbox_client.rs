use async_trait::async_trait;
use mpsc::UnboundedSender;
use oneshot::Sender;
use tokio::sync::{mpsc, mpsc::UnboundedReceiver, oneshot};

use crate::{
    common_infrastructure::{
        balance::TokenBalance,
        datafeed::event::MarketEvent,
        order::{Cancelled, Open, Order, Pending},
    },
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade,
    AccountEvent, ClientExecution, ExchangeVariant, ExecutionError, RequestCancel, RequestOpen,
};

#[derive(Debug)]
pub struct SandBoxClient
{
    pub local_timestamp: i64,
    pub request_tx: UnboundedSender<SandBoxClientEvent>, // NOTE 这是向模拟交易所端发送信号的发射器。注意指令格式是SandBoxClientEvent
    pub strategy_signal_rx: UnboundedReceiver<SandBoxClientEvent>, // NOTE 这是从策略收取信号的接收器。注意指令格式是SandBoxClientEvent
}

// NOTE 模拟交易所客户端可向模拟交易所发送的命令
// 定义类型别名以简化复杂的类型
type OpenOrderResults = Vec<Result<Order<Pending>, ExecutionError>>;
type CancelOrderResults = Vec<Result<Order<Cancelled>, ExecutionError>>;
type RequestOpenOrders = (Vec<Order<RequestOpen>>, Sender<OpenOrderResults>);
type RequestCancelOrders = (Vec<Order<RequestCancel>>, Sender<CancelOrderResults>);

// 模拟交易所客户端可向模拟交易所发送的命令
#[derive(Debug)]
pub enum SandBoxClientEvent
{
    FetchMarketEvent(MarketEvent<ClickhousePublicTrade>),
    FetchOrdersOpen(Sender<Result<Vec<Order<Open>>, ExecutionError>>),
    FetchBalances(Sender<Result<Vec<TokenBalance>, ExecutionError>>),
    OpenOrders(RequestOpenOrders),
    CancelOrders(RequestCancelOrders),
    CancelOrdersAll(Sender<Result<Vec<Order<Cancelled>>, ExecutionError>>),
}

#[async_trait]
impl ClientExecution for SandBoxClient
{
    // in our case the 'optional' config parameter in the sandbox exchange is an UnboundedSender
    type Config = (UnboundedSender<SandBoxClientEvent>, UnboundedReceiver<SandBoxClientEvent>);

    // very naturally, the client's kind is determined by and aligned the exchange.
    const CLIENT_KIND: ExchangeVariant = ExchangeVariant::SandBox;

    async fn init(config: Self::Config, _: UnboundedSender<AccountEvent>, local_timestamp: i64) -> Self
    {
        // 从 config 元组中解构出 request_tx 和 request_rx
        let (request_tx, request_rx) = config;

        // 使用 request_tx 和 request_rx 初始化 SandBoxClient
        Self { request_tx,
               strategy_signal_rx: request_rx,
               local_timestamp }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        self.request_tx
            .send(SandBoxClientEvent::FetchOrdersOpen(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send FetchOrdersOpen request");
        // 从模拟交易所接收开放订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive FetchOrdersOpen response")
    }

    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExecutionError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        self.request_tx
            .send(SandBoxClientEvent::FetchBalances(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send FetchBalances request");
        // 从模拟交易所接收账户余额的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive FetchBalances response")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Pending>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        self.request_tx
            .send(SandBoxClientEvent::OpenOrders((open_requests, response_tx)))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send OpenOrders request");
        // 从模拟交易所接收开启订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive OpenOrders response")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExecutionError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        self.request_tx
            .send(SandBoxClientEvent::CancelOrders((cancel_requests, response_tx)))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send CancelOrders request");
        // 从模拟交易所接收取消订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive CancelOrders response")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExecutionError>
    {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        self.request_tx
            .send(SandBoxClientEvent::CancelOrdersAll(response_tx))
            .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to send CancelOrdersAll request");
        // 从模拟交易所接收取消所有订单的响应。
        response_rx.await
                   .expect("[UniLinkExecution] : Sandbox exchange is currently offline - Failed to receive CancelOrdersAll response")
    }
}
