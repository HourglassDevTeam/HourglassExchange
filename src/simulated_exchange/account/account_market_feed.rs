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
    simulated_exchange::account::account_market_feed::DataStream::{Historical, Live},
};

pub type StreamID = String; //  CUSTOMISABLE Identifier for each stream

pub struct AccountDataStreams<Event>
    where Event: Clone + Send + Sync + 'static
{
    pub streams: HashMap<StreamID, DataStream<Event>>,
}

impl<Event> Debug for AccountDataStreams<Event> where Event: Debug + Clone + Send + Sync + 'static
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.debug_struct("AccountMarketStreams").field("streams", &self.streams.keys().collect::<Vec<_>>()).finish()
    }
}

impl<Event> AccountDataStreams<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn new() -> Self
    {
        Self { streams: HashMap::new() }
    }

    pub fn add_stream(&mut self, id: StreamID, stream: DataStream<Event>)
    {
        self.streams.insert(id, stream);
    }
}




pub enum DataStream<Event>
    where Event: Clone + Send + Sync + 'static
{
    Live(LiveFeed<Event>),
    Historical(HistoricalFeed<Event>),
}

impl<Event> Stream for DataStream<Event> where Event: Clone + Send + Sync + 'static
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

impl<Event> DataStream<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Event> + Send)>
    {
        match self {
            | Live(feed) => feed.poll_next(),
            | Historical(feed) => feed.poll_next(),
        }
    }
}
