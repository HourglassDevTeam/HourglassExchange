use std::{fmt, fmt::Debug, pin::Pin};

use futures_core::Stream;

use crate::{
    common_skeleton::datafeed::{historical::HistoricalFeed, live::LiveFeed},
    error::ExecutionError,
    simulated_exchange::account::account_market_feed::StreamKind::{Historical, Live},
};

pub struct AccountMarketFeed<Event>
    where Event: Clone + Send + Sync + 'static
{
    pub stream_kind_name: &'static str,
    pub data_stream: StreamKind<Event>,
}

impl<Event> Debug for AccountMarketFeed<Event> where Event: Debug + Clone + Send + Sync + 'static
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.debug_struct("AccountMarketFeed").field("atomic_id", &self.stream_kind_name).finish()
    }
}

impl<Event> AccountMarketFeed<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn new(stream: StreamKind<Event>) -> Self
    {
        Self { stream_kind_name: match stream {
                   | Live(_) => "LiveFeed",
                   | Historical(_) => "HistoricalFeed",
               },
               data_stream: stream }
    }
}

// add enum StreamKind for AccountMarketFeed to choose
pub enum StreamKind<Event>
    where Event: Clone + Send + Sync + 'static
{
    Live(LiveFeed<Event>),
    Historical(HistoricalFeed<Event>),
}

impl<Event> StreamKind<Event> where Event: Clone + Send + Sync + 'static
{
    pub fn poll_next(&mut self) -> Pin<&mut (dyn Stream<Item = Result<Event, ExecutionError>> + Send)>
    {
        match self {
            | Live(feed) => feed.poll_next(),
            | Historical(feed) => feed.poll_next(),
        }
    }
}
