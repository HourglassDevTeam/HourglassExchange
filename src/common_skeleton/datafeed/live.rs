use std::{fmt::Debug, pin::Pin};

use futures::{Stream, StreamExt};
use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;

use crate::{
    data_subscriber::{
        connector::Connector,
        socket_error::SocketError,
        subscriber::{Identifier, SubKind, WebSocketSubscriber},
        Subscriber,
    },
    simulated_exchange::account::account_market_feed::Subscription,
};


/// Live feed for events.
pub struct LiveFeed<Event>
{
    pub(crate) stream: UnboundedReceiver<Event>,
}

impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static
{
    pub fn poll_next(&mut self) -> UnboundedReceiver<Event>
    {
        self.stream.as_mut()
    }
}

impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub async fn new<Exchange, SubscriptionKind>(subscriptions: &[Subscription<Exchange, SubscriptionKind>]) -> Result<Self, SocketError>
                                                 where Exchange: Connector + Send + Sync,
                                                       SubscriptionKind: SubKind + Send + Sync,
                                                       Subscription<Exchange, SubscriptionKind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>
    {
        let (websocket, _instrument_map) = WebSocketSubscriber::subscribe(subscriptions).await?;
        let stream = websocket.map(|msg| {
            Event::parse_ws(msg)
        })
            .boxed();

        Ok(Self { stream })
    }
}