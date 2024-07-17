use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use futures_core::Stream;
use tokio::sync::RwLock;

use crate::common_skeleton::datafeed::{FeedStatus, historical::HistoricalFeed, MarketFeedDistributor};
use crate::common_skeleton::datafeed::live::LiveFeed;
use crate::error::ExecutionError;

#[derive(Debug)]
pub struct AccountMarketFeed<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub atomic_id: AtomicU64,
    pub data_stream: Arc<RwLock<Pin<Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>>>>,
}

impl<Event> AccountMarketFeed<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub fn new(stream: Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>) -> Self {
        Self {
            atomic_id: AtomicU64::new(0),
            data_stream: Arc::new(RwLock::new(Box::pin(stream))),
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
pub enum FeedKind<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    LiveFeed(Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>),
    HistoricalFeed(Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>),
}

impl<Event> MarketFeedDistributor<Event> for FeedKind<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    fn fetch_next(&mut self) -> Pin<Box<dyn Stream<Item = Result<Event, ExecutionError>> + Send>> {
        match self {
            FeedKind::LiveFeed(stream) => stream.clone(),
            FeedKind::HistoricalFeed(stream) => stream.clone(),
        }
    }
}
