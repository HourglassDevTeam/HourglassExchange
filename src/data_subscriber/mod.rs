use std::collections::HashMap;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::common_skeleton::instrument::Instrument;
use crate::data_subscriber::mapper::SubscriptionMapper;
use crate::data_subscriber::socket_error::SocketError;
use crate::data_subscriber::subscriber::SubKind;
use crate::simulated_exchange::account::account_market_feed::Subscription;

pub mod connector;
pub mod socket_error;
pub mod subscriber;
pub mod validator;
mod mapper;

#[derive(Debug)]
pub struct SubscriptionId(pub String);

pub struct SubscriptionMeta
{
    /// `HashMap` containing the mapping between a [`SubscriptionId`] and
    /// it's associated Cerebro [`Instrument`].
    pub instrument_map: Map<Instrument>,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub subscriptions: Vec<WsMessage>,
}

pub struct Map<T>(pub HashMap<SubscriptionId, T>);
pub type WebSocket = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;


#[async_trait]
pub trait Subscriber {
    type SubMapper: SubscriptionMapper;
    async fn subscribe<Kind>(subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, Map<Instrument>), SocketError>
                                       where Kind: SubKind + Send + Sync;
}
