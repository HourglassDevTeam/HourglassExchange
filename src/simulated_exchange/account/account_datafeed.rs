use std::sync::atomic::{AtomicU64, Ordering};

use crate::common_skeleton::datafeed::{FeedStatus, historical::HistoricalFeed, MarketFeedDistributor};
use crate::common_skeleton::datafeed::live::LiveFeed;

#[derive(Debug)]
pub struct AccountMarketFeed<Iter,Event,>
where
    Event: Clone,
    Iter: Iterator<Item = Event> + Clone,
    FeedKind<Iter,Event>: MarketFeedDistributor<Event>,
{
    pub atomic_id: AtomicU64,
    pub data: FeedKind<Iter,Event>,
}

impl<Iter,Event> AccountMarketFeed<Iter,Event>
where
    Event: Clone,
    Iter: Iterator<Item = Event> + Clone,
    FeedKind<Iter,Event>: MarketFeedDistributor<Event>,
{
    pub fn new(data: FeedKind<Iter,Event>) -> Self {
        Self {
            atomic_id: AtomicU64::new(0),
            data,
        }
    }

    pub fn increment_batch_id(&self) {
        self.atomic_id.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_batch_id(&self) -> u64 {
        self.atomic_id.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum FeedKind<Iter,Event>
where
    Event: Clone,
    Iter: Iterator<Item = Event> + Clone,
{
    LiveFeed(LiveFeed<Event>),
    HistoricalFeed(HistoricalFeed<Iter, Event>),
}

impl<Iter,Event> MarketFeedDistributor<Event> for FeedKind<Iter,Event>
where
    Event: Clone,
    Iter: Iterator<Item = Event> + Clone,
    LiveFeed<Event>: MarketFeedDistributor<Event>,
    HistoricalFeed<Iter, Event>: MarketFeedDistributor<Event>,
{
    fn fetch_next(&mut self) -> FeedStatus<Event> {
        match self {
            FeedKind::LiveFeed(feed) => feed.fetch_next(),
            FeedKind::HistoricalFeed(feed) => feed.fetch_next(),
        }
    }
}
