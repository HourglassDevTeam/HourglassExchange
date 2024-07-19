use std::{
    collections::HashMap,
    fmt,
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;

use crate::{
    common_skeleton::datafeed::{historical::HistoricalFeed, live::LiveFeed},
    simulated_exchange::account::account_market_feed::MarketStream::{Historical, Live},
};

// Define a unique identifier for the streams
pub type StreamID = String;

pub struct AccountMarketStreams<Event>
    where Event: Clone + Send + Sync + 'static
{
    pub streams: HashMap<StreamID, MarketStream<Event>>,
}

impl<Event> Debug for AccountMarketStreams<Event> where Event: Debug + Clone + Send + Sync + 'static
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.debug_struct("AccountMarketStreams").field("streams", &self.streams.keys().collect::<Vec<_>>()).finish()
    }
}

impl<Event> AccountMarketStreams<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn new() -> Self
    {
        Self { streams: HashMap::new() }
    }

    pub fn add_stream(&mut self, id: StreamID, stream: MarketStream<Event>)
    {
        self.streams.insert(id, stream);
    }
}

// add enum StreamKind for AccountMarketFeed to choose
pub enum MarketStream<Event>
    where Event: Clone + Send + Sync + 'static
{
    Live(LiveFeed<Event>),
    Historical(HistoricalFeed<Event>),
}

impl<Event> Stream for MarketStream<Event> where Event: Clone + Send + Sync + 'static
{
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>
    {
        match self.get_mut() {
            | Historical(feed) => Pin::new(&mut feed.stream).poll_next(cx),
            | Live(feed) => Pin::new(&mut feed.stream).poll_next(cx),
        }
    }
}

impl<Event> MarketStream<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Event> + Send)>
    {
        match self {
            | Live(feed) => feed.poll_next(),
            | Historical(feed) => feed.poll_next(),
        }
    }
}
