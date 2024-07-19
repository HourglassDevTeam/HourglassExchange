use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{connector::Connector, socket_error::SocketError, Map, SubscriptionMeta},
    simulated_exchange::account::account_market_feed::Subscription,
};
use async_trait::async_trait;
use std::fmt::Debug;
use tokio_tungstenite::tungstenite::WebSocket;
use tracing::{debug, info};

pub struct WebSocketSubscriber;
pub trait SubKind
    where Self: Debug + Clone
{
    type Event: Debug;
}

pub struct ExchangeSub<Channel, Market>
{
    /// Type that defines how to translate a Cerebro [`Subscription`] into an exchange specific
    /// channel to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](super::binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](super::kraken::channel::KrakenChannel)
    pub channel: Channel,

    /// Type that defines how to translate a Cerebro [`Subscription`] into an exchange specific
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

    async fn subscribe<Kind>(subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, Map<Instrument>), SocketError>
        where Kind: SubKind + Send + Sync
    {
        // Define variables for logging ergonomics
        let exchange = Exchange::ID;

        let url = Exchange::url()?;

        debug!(%exchange, %url, ?subscriptions, "subscribing to WebSocket");

        // Connect to exchange
        let mut websocket = connect(url).await?;

        debug!(%exchange, ?subscriptions, "connected to WebSocket");

        // Map &[Subscription<Kind>] to SubscriptionMeta
        let SubscriptionMeta { instrument_map, subscriptions } = Self::SubMapper::map::<Kind>(subscriptions);

        // Send Subscriptions over WebSocket
        for subscription in subscriptions {
            debug!(%exchange, payload = ?subscription, "sending exchange subscription");

            websocket.send(subscription).await?;
        }

        // Validate Subscription responses
        let map = Exchange::SubValidator::validate::<Kind>(instrument_map, &mut websocket).await?;

        info!(%exchange, "subscribed to WebSocket");

        Ok((websocket, map))
    }
}
