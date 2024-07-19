use std::{fmt::Debug, pin::Pin};

use crate::{
    data_subscriber::{connector::Connector, socket_error::SocketError, subscriber::WebSocketSubscriber},
    simulated_exchange::account::account_market_feed::Subscription,
};
use futures::Stream;
use crate::data_subscriber::subscriber::SubKind;

pub struct LiveFeed<Event>
{
    pub(crate) stream: Pin<Box<dyn Stream<Item = Event> + Send>>,
}

impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Event> + Send)>
    {
        self.stream.as_mut()
    }
}

impl<Event> LiveFeed<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    pub async fn new<Exchange, Kind>(subscriptions: &[Subscription<Kind>]) -> Result<Self, SocketError>
        where Exchange: Connector + Send + Sync,
              Kind: SubKind + Send + Sync
    {
        let (websocket, _instrument_map) = WebSocketSubscriber::subscribe(subscriptions).await?;
        let stream = websocket.map(|msg| {
                                  // 将WebSocket消息解析为事件
                                  // 这里你需要根据你的业务逻辑进行实现
                                  Event::from_websocket_message(msg)
                              })
                              .boxed();

        Ok(Self { stream })
    }
}
