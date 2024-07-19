use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    data_subscriber::{
        connector::Connector,
        SubscriptionMap,
        subscriber::{ExchangeSub, SubKind}, SubscriptionMeta,
    },
    simulated_exchange::account::account_market_feed::Subscription,
};

/// Defines how to map a collection of  [`Subscription`]s into exchange specific
/// [`SubscriptionMeta`], containing subscription payloads that are sent to the exchange.

pub trait SubscriptionMapper
{
    fn map<Kind>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta
        where Kind: SubKind;
}

/// Standard [`SubscriptionMapper`] for
/// [`WebSocket`](cerebro_integration::protocol::websocket::WebSocket)s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]

pub struct WebSocketSubMapper;

impl SubscriptionMapper for WebSocketSubMapper
{
    /// 将订阅数组映射到 `SubscriptionMeta`。
    fn map<Kind>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta
        where Kind: SubKind
    {
        // 分配 SubscriptionIds HashMap，用于跟踪每个操作订阅的标识符
        let mut instrument_map = SubscriptionMap(HashMap::with_capacity(subscriptions.len()));

        // 将订阅映射成特定交易所的订阅
        let exchange_subs = subscriptions.iter()
                                         .map(|subscription| {
                                             // 将 Subscription 转换为特定交易所的订阅
                                             let exchange_sub = ExchangeSub::new(subscription);

                                             // 确定与此特定交易所订阅关联的 SubscriptionId
                                             let subscription_id = exchange_sub.id();

                                             // 使用 ExchangeSub SubscriptionId 作为此订阅的链接
                                             instrument_map.0.insert(subscription_id, subscription.instrument.clone());

                                             exchange_sub
                                         })
                                         // 收集为 ExchangeSub 向量
                                         .collect::<Vec<ExchangeSub<Exchange::Channel, Exchange::Market>>>();

        // 构建 WebSocket 消息订阅请求
        let subscriptions = Exchange::requests(exchange_subs);

        // 返回包含 instrument_map 和 subscriptions 的 SubscriptionMeta
        SubscriptionMeta { instrument_map, subscriptions }
    }
}
