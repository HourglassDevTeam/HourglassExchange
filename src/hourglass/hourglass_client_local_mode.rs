use crate::hourglass_log::{info, warn};
use async_trait::async_trait;
use mpsc::UnboundedSender;
use oneshot::Sender;
use tokio::sync::{mpsc, mpsc::UnboundedReceiver, oneshot};
use HourglassClientEvent::{CancelOrders, CancelOrdersAll, FetchOrdersOpen, FetchTokenBalances, OpenOrders};

use crate::{
    common::{
        account_positions::{AccountPositions, Position, PositionConfig},
        balance::TokenBalance,
        instrument::Instrument,
        order::{
            states::{cancelled::Cancelled, open::Open, request_cancel::RequestCancel},
            Order,
        },
        token::Token,
    },
    hourglass::{clickhouse_api::datatype::clickhouse_trade_data::MarketTrade, config_request::ConfigurationRequest},
    network::login::{LoginRequest, LogoutRequest, RegisterRequest},
    AccountEvent, ClientExecution, Exchange, ExchangeError, RequestOpen,
};

#[derive(Debug)]
pub struct HourglassClient
{
    pub client_event_tx: UnboundedSender<HourglassClientEvent>,
    pub market_event_rx: UnboundedReceiver<MarketTrade>,
    // pub account_event_rx: UnboundedReceiver<AccountEvent>,
}

// NOTE 模拟交易所客户端可向模拟交易所发送的命令
// 定义类型别名以简化复杂的类型
pub type OpenOrderResults = Vec<Result<Order<Open>, ExchangeError>>;
pub type CancelOrderResults = Vec<Result<Order<Cancelled>, ExchangeError>>;
pub type ConfigureInstrumentsResults = Vec<Result<PositionConfig, ExchangeError>>;
pub type RequestOpenOrders = (Vec<Order<RequestOpen>>, Sender<OpenOrderResults>);
pub type RequestCancelOrders = (Vec<Order<RequestCancel>>, Sender<CancelOrderResults>);
pub type DepositResults = Result<Vec<TokenBalance>, ExchangeError>;
pub type DepositRequest = (Vec<(Token, f64)>, Sender<DepositResults>);

// 模拟交易所客户端可向模拟交易所发送的命令
#[derive(Debug)]
pub enum HourglassClientEvent
{
    DepositTokens(DepositRequest),
    FetchOrdersOpen(Sender<Result<Vec<Order<Open>>, ExchangeError>>),
    FetchTokenBalances(Sender<Result<Vec<TokenBalance>, ExchangeError>>),
    FetchTokenBalance(Token, Sender<Result<TokenBalance, ExchangeError>>),
    FetchLongPosition(Instrument, Sender<Result<Option<Position>, ExchangeError>>),
    FetchShortPosition(Instrument, Sender<Result<Option<Position>, ExchangeError>>),
    FetchAllPositions(Sender<Result<AccountPositions, ExchangeError>>),
    OpenOrders(RequestOpenOrders),
    CancelOrders(RequestCancelOrders),
    CancelOrdersAll(Sender<Result<Vec<Order<Cancelled>>, ExchangeError>>),
    ConfigureInstruments(Vec<ConfigurationRequest>, Sender<ConfigureInstrumentsResults>),
    LetItRoll, // Tell the system to send the next datafeed.
    Register(RegisterRequest),
    Login(LoginRequest),
    Logout(LogoutRequest),
}

#[async_trait]
impl ClientExecution for HourglassClient
{
    const CLIENT_KIND: Exchange = Exchange::Hourglass;

    type Config = (UnboundedSender<HourglassClientEvent>, UnboundedReceiver<MarketTrade>);

    async fn init(config: Self::Config, _: UnboundedSender<AccountEvent>) -> Self
    {
        // 从 config 元组中解构出 request_tx 和 market_event_rx
        let (request_tx, market_event_rx) = config;

        // 使用 request_tx 和 market_event_rx 初始化 HourglassClient
        Self { client_event_tx: request_tx,
               market_event_rx }
    }

    async fn fetch_orders_open(&self) -> Result<Vec<Order<Open>>, ExchangeError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取开放订单的请求。
        self.client_event_tx
            .send(FetchOrdersOpen(response_tx))
            .expect("Hourglass exchange is currently offline - Failed to send FetchOrdersOpen request");
        // 从模拟交易所接收开放订单的响应。
        response_rx.await.expect("Hourglass exchange is currently offline - Failed to receive FetchOrdersOpen response")
    }

    async fn fetch_balances(&self) -> Result<Vec<TokenBalance>, ExchangeError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送获取账户余额的请求。
        self.client_event_tx
            .send(FetchTokenBalances(response_tx))
            .expect("Hourglass exchange is currently offline - Failed to send FetchBalances request");
        // 从模拟交易所接收账户余额的响应。
        response_rx.await.expect("Hourglass exchange is currently offline - Failed to receive FetchBalances response")
    }

    //  FetchAllPositions 的实现
    async fn fetch_all_positions(&self) -> Result<AccountPositions, ExchangeError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        self.client_event_tx
            .send(HourglassClientEvent::FetchAllPositions(response_tx))
            .expect("[HourglassClient] : Failed to send FetchAllPositions request");
        response_rx.await.expect("[HourglassClient] : Failed to receive FetchAllPositions response")
    }

    //  FetchLongPosition 的实现
    async fn fetch_long_position(&self, instrument: Instrument) -> Result<Option<Position>, ExchangeError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        self.client_event_tx
            .send(HourglassClientEvent::FetchLongPosition(instrument, response_tx))
            .expect("[HourglassClient] : Failed to send FetchLongPosition request");
        response_rx.await.expect("[HourglassClient] : Failed to receive FetchLongPosition response")
    }

    //  FetchShortPosition 的实现
    async fn fetch_short_position(&self, instrument: Instrument) -> Result<Option<Position>, ExchangeError>
    {
        let (response_tx, response_rx) = oneshot::channel();
        self.client_event_tx
            .send(HourglassClientEvent::FetchShortPosition(instrument, response_tx))
            .expect("[HourglassClient] : Failed to send FetchShortPosition request");
        response_rx.await.expect("[HourglassClient] : Failed to receive FetchShortPosition response")
    }

    async fn open_orders(&self, open_requests: Vec<Order<RequestOpen>>) -> Vec<Result<Order<Open>, ExchangeError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送开启订单的请求。
        self.client_event_tx
            .send(OpenOrders((open_requests, response_tx)))
            .expect("Hourglass exchange is currently offline - Failed to send OpenOrders request");
        // 从模拟交易所接收开启订单的响应。
        response_rx.await.expect("Hourglass exchange is currently offline - Failed to receive OpenOrders response")
    }

    async fn cancel_orders(&self, cancel_requests: Vec<Order<RequestCancel>>) -> Vec<Result<Order<Cancelled>, ExchangeError>>
    {
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消订单的请求。
        self.client_event_tx
            .send(CancelOrders((cancel_requests, response_tx)))
            .expect("Hourglass exchange is currently offline - Failed to send CancelOrders request");
        // 从模拟交易所接收取消订单的响应。
        response_rx.await.expect("Hourglass exchange is currently offline - Failed to receive CancelOrders response")
    }

    async fn cancel_orders_all(&self) -> Result<Vec<Order<Cancelled>>, ExchangeError>
    {
        // 创建一个 oneshot 通道以与模拟交易所通信。
        let (response_tx, response_rx) = oneshot::channel();
        // 向模拟交易所发送取消所有订单的请求。
        self.client_event_tx
            .send(CancelOrdersAll(response_tx))
            .expect("Hourglass exchange is currently offline - Failed to send CancelOrdersAll request");
        // 从模拟交易所接收取消所有订单的响应。
        response_rx.await.expect("Hourglass exchange is currently offline - Failed to receive CancelOrdersAll response")
    }

    // 实现 DepositTokens 的处理逻辑
    async fn deposit_tokens(&self, deposits: Vec<(Token, f64)>) -> Result<Vec<TokenBalance>, ExchangeError>
    {
        info!("begin to deposit tokens: {:?}", deposits);
        let (response_tx, response_rx) = oneshot::channel();
        self.client_event_tx
            .send(HourglassClientEvent::DepositTokens((deposits, response_tx)))
            .expect("Failed to send DepositTokens request");
        response_rx.await.expect("Failed to receive DepositTokens response")
    }

    // 发送 LetItRoll 命令的函数
    async fn let_it_roll(&self) -> Result<(), ExchangeError>
    {
        // Try sending the LetItRoll event and handle the case where the channel is closed
        if let Err(err) = self.client_event_tx.send(HourglassClientEvent::LetItRoll) {
            // Channel is closed, so handle it gracefully instead of panicking
            warn!("Failed to send LetItRoll request: {:?}", err);
            return Err(ExchangeError::Hourglass("Exchange is currently offline".into()));
        }

        info!("Sent LetItRoll command successfully");
        Ok(())
    }
}

impl HourglassClient
{
    pub async fn listen_for_market_data(&mut self) -> Option<MarketTrade>
    {
        if let Some(market_event) = self.market_event_rx.recv().await {
            // Process the received market data event
            info!("Received market event: {:?}", market_event);
            return Some(market_event);
        }
        None // Return None if there are no events
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_fetch_orders_open()
    {
        // 创建通道，用于请求和响应通信
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        let (_market_tx, market_rx) = mpsc::unbounded_channel();

        let client = HourglassClient { client_event_tx: request_tx.clone(),
                                       market_event_rx: market_rx };

        // 启动一个异步任务来调用客户端的 fetch_orders_open 方法
        let client_task = tokio::spawn(async move {
            // 等待 fetch_orders_open 方法完成，并期待没有错误发生
            let orders = client.fetch_orders_open().await.expect("fetch_orders_open failed");
            // 断言获取到的订单列表是空的
            assert!(orders.is_empty(), "Expected an empty list of orders");
        });

        // 等待客户端发送 FetchOrdersOpen 请求事件
        // 预期客户端会发送一个获取打开订单的请求
        let request_event = request_rx.recv().await.expect("Expected FetchOrdersOpen event");

        // 匹配接收到的事件，确认它是 FetchOrdersOpen 类型
        if let FetchOrdersOpen(tx) = request_event {
            // 使用 oneshot 通道的发送者发送一个模拟的响应
            // 这里模拟返回一个空的订单列表
            let _ = tx.send(Ok(vec![]));
        }
        else {
            // 如果接收到的事件不是预期的 FetchOrdersOpen 类型，使用 panic 使测试失败
            panic!("Received unexpected event type");
        }

        // 等待客户端任务完成，确保 fetch_orders_open 方法已成功执行
        client_task.await.expect("Client task should complete successfully");
    }

    #[tokio::test]
    async fn test_cancel_orders_all()
    {
        // 创建一个模拟的 HourglassClientEvent 发射器和接收器
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        // let (_market_event_tx, market_event_rx) = mpsc::unbounded_channel();
        let (_response_tx, _response_rx) = oneshot::channel::<Result<Vec<Order<Cancelled>>, ExchangeError>>();
        let (_market_tx, market_rx) = mpsc::unbounded_channel();

        // 初始化 HourglassClient
        let client = HourglassClient { client_event_tx: request_tx.clone(),
                                       market_event_rx: market_rx };

        // 启动一个异步任务来调用客户端的 cancel_orders_all 方法
        let client_task = tokio::spawn(async move {
            let result = client.cancel_orders_all().await;
            info!("Client received response: {:?}", result); // 打印客户端接收到的响应
            assert!(result.is_ok(), "Expected a successful result");
            assert!(result.unwrap().is_empty(), "Expected an empty list of cancelled orders");
        });

        // 模拟从 HourglassClientEvent 接收器获取 CancelOrdersAll 事件
        if let Some(CancelOrdersAll(tx)) = request_rx.recv().await {
            // info!("Received CancelOrdersAll event");

            // 发送一个空的取消订单列表作为响应
            let response = Ok(vec![]);
            // info!("Response being sent: {:?}", response); // 打印将要发送的响应

            // 发送响应，并确认是否成功发送
            if tx.send(response).is_ok() {
                info!("Response sent successfully");
            }
            else {
                info!("Failed to send CancelOrdersAll response");
            }
        }
        else {
            panic!("Did not receive CancelOrdersAll event");
        }

        // 确保客户端任务完成
        client_task.await.expect("Client task should complete successfully");
    }
}
