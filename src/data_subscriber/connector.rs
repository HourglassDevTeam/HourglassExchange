use std::{collections::HashMap, fmt::Debug, time::Duration};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::Subscriber;
use url::Url;

use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{
        socket_error::SocketError,
        subscriber::ExchangeSub,
        SubscriptionId,
        validator::{SubscriptionValidator, Validator}, WsMessage,
    },
    ExchangeKind,
};

#[derive(Debug)]
pub struct PingInterval
{
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

pub trait Connector
    where Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized
{
    /// Unique identifier for the exchange server being connected with.

    const ID: ExchangeKind;

    /// Type that defines how to translate a
    /// [`Subscription`](crate::subscription::Subscription) into an exchange specific channel
    /// to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](kraken::channel::KrakenChannel)

    type Channel: AsRef<str>;

    /// Type that defines how to translate a
    /// [`Subscription`](crate::subscription::Subscription) into an exchange specific market that
    /// can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](kraken::market::KrakenMarket)

    type Market: AsRef<str>;

    /// [`Subscriber`] type that establishes a connection with the exchange server, and actions
    /// [`Subscription`](crate::subscription::Subscription)s over the socket.

    type Subscriber: Subscriber;

    /// [`SubscriptionValidator`] type that listens to responses from the exchange server and
    /// validates if the actioned [`Subscription`](crate::subscription::Subscription)s were
    /// successful.

    type SubValidator: SubscriptionValidator;

    /// Deserialisable type that the [`Self::SubValidator`] expects to receive from the exchange server in
    /// response to the [`Subscription`](crate::subscription::Subscription) [`Self::requests`]
    /// sent over the [`WebSocket`](cerebro_integration::protocol::websocket::WebSocket). Implements
    /// [`Validator`](cerebro_integration::Validator) in order to determine if [`Self`]
    /// communicates a successful [`Subscription`](crate::subscription::Subscription) outcome.

    type SubResponse: Validator + Debug + DeserializeOwned;

    /// Base [`Url`] of the exchange server being connected with.

    fn url() -> Result<Url, SocketError>;

    /// Defines [`PingInterval`] of custom application-level
    /// [`WebSocket`](cerebro_integration::protocol::websocket::WebSocket) pings for the exchange
    /// server being connected with.
    ///
    /// Defaults to `None`, meaning that no custom pings are sent.

    fn ping_interval() -> Option<PingInterval>
    {
        None
    }

    /// Defines how to translate a collection of [`ExchangeSub`]s into the [`WsMessage`]
    /// subscription payloads sent to the exchange server.

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// Number of [`Subscription`](crate::subscription::Subscription) responses expected from the
    /// exchange server in responses to the requests send. Used to validate all
    /// [`Subscription`](crate::subscription::Subscription)s were accepted.

    fn expected_responses(map: &SubMap<Instrument>) -> usize
    {
        map.0.len()
    }

    /// Expected [`Duration`] the [`SubscriptionValidator`] will wait to receive all success
    /// responses to actioned [`Subscription`](crate::subscription::Subscription) requests.

    fn subscription_timeout() -> Duration
    {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

pub struct SubMap<T>(pub HashMap<SubscriptionId, T>);

impl<T> FromIterator<(SubscriptionId, T)> for SubMap<T>
{
    fn from_iter<Iter>(iter: Iter) -> Self
        where Iter: IntoIterator<Item = (SubscriptionId, T)>
    {
        Self(iter.into_iter().collect::<HashMap<SubscriptionId, T>>())
    }
}

impl<T> SubMap<T>
{
    /// Find the `T` associated with the provided [`SubscriptionId`].

    pub fn find(&self, id: &SubscriptionId) -> Result<T, SocketError>
        where T: Clone
    {
        self.0.get(id).cloned().ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }

    /// Find the mutable reference to `T` associated with the provided [`SubscriptionId`].

    pub fn find_mut(&mut self, id: &SubscriptionId) -> Result<&mut T, SocketError>
    {
        self.0.get_mut(id).ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}
