use std::fmt::Debug;

use async_trait::async_trait;
use tokio_tungstenite::tungstenite::{connect};
use tracing::{debug, info};

use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{connector::Connector, Map, mapper::WebSocketSubMapper, socket_error::SocketError, Subscriber, SubscriptionMeta},
    simulated_exchange::account::account_market_feed::Subscription,
};
use crate::data_subscriber::WebSocket;

pub struct WebSocketSubscriber;
pub trait SubKind
    where Self: Debug + Clone
{
    type Event: Debug;
}

pub struct ExchangeSub<Channel, Market>
{
    /// Type that defines how to translate a  [`Subscription`] into an exchange specific
    /// channel to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](super::binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](super::kraken::channel::KrakenChannel)
    pub channel: Channel,

    /// Type that defines how to translate a  [`Subscription`] into an exchange specific
    /// market that can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](super::binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](super::kraken::market::KrakenMarket)
    pub market: Market,
}


#[async_trait]
impl Subscriber for WebSocketSubscriber
{
    type SubMapper = WebSocketSubMapper;

    /// 订阅方法
    /// 通过 WebSocket 连接到交易所，并发送订阅请求。
    /// 返回包含 WebSocket 和订阅映射的结果，或返回 `SocketError`。
    async fn subscribe<Kind>(subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, Map<Instrument>), SocketError>
                             where Kind: SubKind + Send + Sync
    {
        // 定义变量用于日志记录
        let exchange = Exchange::ID;

        // 获取交易所的 WebSocket URL
        let url = Exchange::url()?;

        // 记录订阅日志
        debug!(%exchange, %url, ?subscriptions, "subscribing to WebSocket");

        // 连接到交易所
        let mut websocket = connect(url).await?;

        // 记录连接成功日志
        debug!(%exchange, ?subscriptions, "connected to WebSocket");

        // 将 &[Subscription<Kind>] 映射到 SubscriptionMeta
        let SubscriptionMeta { instrument_map, subscriptions } = Self::SubMapper::map::<Kind>(subscriptions);

        // 通过 WebSocket 发送订阅请求
        for subscription in subscriptions {
            debug!(%exchange, payload = ?subscription, "sending exchange subscription");

            websocket.send(subscription).await?;
        }

        // 验证订阅响应
        let map = Exchange::SubValidator::validate::<Kind>(instrument_map, &mut websocket).await?;

        // 记录订阅成功日志
        info!(%exchange, "subscribed to WebSocket");

        // 返回 WebSocket 和订阅映射
        Ok((websocket, map))
    }