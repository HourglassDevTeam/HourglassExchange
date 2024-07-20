use std::{collections::HashMap, fmt::Debug, time::Duration};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::Subscriber;
use url::Url;

use crate::{
    common_skeleton::instrument::Instrument,
    data_subscriber::{
        socket_error::SocketError,
        subscriber::ExchangeSub,
        validator::{SubscriptionValidator, Validator},
        SubscriptionId, SubscriptionMap, WsMessage,
    },
    ExchangeVariant,
};

/// 表示 Ping 间隔的结构体
#[derive(Debug)]
pub struct PingInterval
{
    pub interval: tokio::time::Interval,
    /// 把 Ping 转化为消息的函数
    pub ping: fn() -> WsMessage,
}

/// 默认订阅超时时间
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

/// 连接器特征，定义了如何与交易所服务器进行连接和通信
pub trait Connector
    where Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized
{
    /// 连接的交易所服务器的唯一标识符
    const ID: ExchangeVariant;

    /// 定义如何将 [`Subscription`](crate::subscription::Subscription) 转换为交易所特定的通道
    type Channel: AsRef<str>;

    /// 定义如何将 [`Subscription`](crate::subscription::Subscription) 转换为交易所特定的市场
    type Market: AsRef<str>;

    /// 建立与交易所服务器连接的订阅者类型
    type Subscriber: Subscriber;

    /// 监听交易所服务器响应并验证订阅是否成功的验证器类型
    type SubValidator: SubscriptionValidator;

    /// 期望从交易所服务器接收的响应类型
    type SubResponse: Validator + Debug + DeserializeOwned;

    /// 返回交易所服务器的基础 URL
    fn url() -> Result<Url, SocketError>;

    /// 定义自定义应用级别 WebSocket ping 的 `PingInterval`
    fn ping_interval() -> Option<PingInterval>
    {
        None
    }

    /// 定义如何将一组 [`ExchangeSub`] 转换为发送到交易所服务器的 [`WsMessage`] 订阅负载
    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// 定义期望从交易所服务器接收到的订阅响应数量
    fn expected_responses(map: &SubscriptionMap<Instrument>) -> usize
    {
        map.0.len()
    }

    /// 定义订阅验证器等待接收所有成功响应的最大时间
    fn subscription_timeout() -> Duration
    {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

/// 用于存储订阅映射的结构体

impl<T> FromIterator<(SubscriptionId, T)> for SubscriptionMap<T>
{
    /// 从迭代器生成 `SubMap` 实例
    fn from_iter<Iter>(iter: Iter) -> Self
        where Iter: IntoIterator<Item = (SubscriptionId, T)>
    {
        Self(iter.into_iter().collect::<HashMap<SubscriptionId, T>>())
    }
}

impl<T> SubscriptionMap<T>
{
    /// 查找与提供的 [`SubscriptionId`] 关联的 `T`
    pub fn find(&self, id: &SubscriptionId) -> Result<T, SocketError>
        where T: Clone
    {
        self.0.get(id).cloned().ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }

    /// 查找与提供的 [`SubscriptionId`] 关联的 `T` 的可变引用
    pub fn find_mut(&mut self, id: &SubscriptionId) -> Result<&mut T, SocketError>
    {
        self.0.get_mut(id).ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}
