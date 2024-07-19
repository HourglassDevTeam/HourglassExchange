use std::fmt::Debug;

use crate::data_subscriber::mapper::SubscriptionMapper;
use async_trait::async_trait;
use tracing::{debug, info};
use futures::SinkExt;
use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{
        connector::Connector, mapper::WebSocketSubMapper, socket_error::SocketError, validator::SubscriptionValidator, Subscriber, SubscriptionId, SubscriptionMap,
        SubscriptionMeta, WebSocket,
    },
    simulated_exchange::account::account_market_feed::Subscription,
};
use crate::data_subscriber::websocket::connect;

pub struct WebSocketSubscriber;
pub trait SubKind
    where Self: Debug + Clone
{
    type Event: Debug;
}

pub struct ExchangeSub<Channel, Market>
{
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](super::binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](super::kraken::channel::KrakenChannel)
    pub channel: Channel,
    /// - [`BinanceMarket("btcusdt")`](super::binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](super::kraken::market::KrakenMarket)
    pub market: Market,
}
pub trait Identifier<T>
{
    fn id(&self) -> T;
}

impl<Channel, Market> Identifier<SubscriptionId> for ExchangeSub<Channel, Market>
    where Channel: AsRef<str>,
          Market: AsRef<str>
{
    fn id(&self) -> SubscriptionId
    {
        SubscriptionId::from(format!("{}|{}", self.channel.as_ref(), self.market.as_ref()))
    }
}

impl<Channel, Market> ExchangeSub<Channel, Market>
    where Channel: AsRef<str>,
          Market: AsRef<str>
{
    /// Construct a new exchange specific [`Self`] with the Cerebro [`Subscription`] provided.

    pub fn new<Exchange, Kind>(sub: &Subscription<Exchange, Kind>) -> Self
        where Subscription<Exchange, Kind>: Identifier<Channel> + Identifier<Market>
    {
        Self { channel: sub.id(),
               market: sub.id() }
    }
}
#[async_trait]
impl Subscriber for WebSocketSubscriber
{
    type SubscriptionMapper = WebSocketSubMapper;

    /// 订阅方法
    /// 通过 WebSocket 连接到交易所，并发送订阅请求。
    /// 返回包含 WebSocket 和订阅映射的结果，或返回 `SocketError`。
    async fn subscribe<Exchange, Kind>(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<(WebSocket, SubscriptionMap<Instrument>), SocketError>
        where Exchange: Connector + Send + Sync,
              Kind: SubKind + Send + Sync,
              Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>
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

        // 将 &[Subscription<Exchange,Kind>] 映射到 SubscriptionMeta
        let SubscriptionMeta { instrument_map, subscriptions } = Self::SubscriptionMapper::map::<Exchange, Kind>(subscriptions);

        // 通过 WebSocket 发送订阅请求
        for subscription in subscriptions {
            debug!(%exchange, payload = ?subscription, "sending exchange subscription");

            websocket.send(subscription).await?;
        }

        // 验证订阅响应
        let map = Exchange::SubValidator::validate::<Exchange, Kind>(instrument_map, &mut websocket).await?;

        // 记录订阅成功日志
        info!(%exchange, "subscribed to WebSocket");

        // 返回 WebSocket 和订阅映射
        Ok((websocket, map))
    }
}
