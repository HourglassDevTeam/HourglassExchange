use std::collections::HashMap;

use async_trait::async_trait;
use tokio::net::TcpStream;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::MaybeTlsStream;

use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{mapper::SubscriptionMapper, socket_error::SocketError, subscriber::SubKind},
    simulated_exchange::account::account_market_feed::Subscription,
};

pub mod connector;
mod mapper;
pub mod socket_error;
pub mod subscriber;
pub mod validator;
mod websocket;

#[derive(Debug)]
pub struct SubscriptionId(pub String);

/// `SubscriptionMeta` 结构体
/// 包含 `HashMap`，用于映射 [`SubscriptionId`] 与其关联的  [`Instrument`]。
/// 还包括一个 `Vec`，用于存储 WebSocket 消息（`WsMessage`）。
pub struct SubscriptionMeta
{
    /// `HashMap`，包含 [`SubscriptionId`] 和它关联的  [`Instrument`] 之间的映射。
    pub instrument_map: Map<Instrument>,
    /// 存储 WebSocket 消息的向量。
    pub subscriptions: Vec<WsMessage>,
}

/// `Map` 结构体
/// 使用泛型参数 `T`，包含一个 `HashMap`，用于存储 [`SubscriptionId`] 与泛型类型 `T` 之间的映射。
pub struct Map<T>(pub HashMap<SubscriptionId, T>);
/// 使用 tokio-tungstenite 库的 [WebSocketStream]，可能是 TLS 或非 TLS 的 TcpStream。
pub type WebSocket = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
/// `Subscriber` 特征
/// 定义了一个异步的订阅方法，用于处理订阅操作。
pub trait Subscriber
{
    /// 关联的订阅映射器类型。
    type SubMapper: SubscriptionMapper;
    async fn subscribe<Kind>(subscriptions: &[Subscription<Kind>]) -> Result<(WebSocket, Map<Instrument>), SocketError>
        where Kind: SubKind + Send + Sync;
}
