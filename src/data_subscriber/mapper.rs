use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    data_subscriber::{
        connector::Connector,
        subscriber::{ExchangeSub, SubKind},
        Map, SubscriptionMeta,
    },
    simulated_exchange::account::account_market_feed::Subscription,
};

/// Defines how to map a collection of Cerebro [`Subscription`]s into exchange specific
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
    fn map<Kind>(subscriptions: &[Subscription<Kind>]) -> SubscriptionMeta
        where Kind: SubKind
    {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut instrument_map = Map(HashMap::with_capacity(subscriptions.len()));

        // Map Cerebro Subscriptions to exchange specific subscriptions
        let exchange_subs = subscriptions.iter()
                                         .map(|subscription| {
                                             // Translate Cerebro Subscription to exchange specific subscription
                                             let exchange_sub = ExchangeSub::new(subscription);

                                             // Determine the SubscriptionId associated with this exchange specific subscription
                                             let subscription_id = exchange_sub.id();

                                             // Use ExchangeSub SubscriptionId as the link to this Cerebro Subscription
                                             instrument_map.0.insert(subscription_id, subscription.instrument.clone());

                                             exchange_sub
                                         })
                                         .collect::<Vec<ExchangeSub<Exchange::Channel, Exchange::Market>>>();

        // Construct WebSocket message subscriptions requests
        let subscriptions = Exchange::requests(exchange_subs);

        SubscriptionMeta { instrument_map, subscriptions }
    }
}
