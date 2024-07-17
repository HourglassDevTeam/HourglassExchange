use std::fmt;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::common_skeleton::datafeed::historical::HistoricalFeed;
use crate::common_skeleton::datafeed::live::LiveFeed;

pub struct AccountMarketFeed<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub atomic_id: AtomicU64,
    pub data_stream: StreamKind<Event>,
}

impl<Event> Debug for AccountMarketFeed<Event>
where
    Event: Debug + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AccountMarketFeed").field("atomic_id", &self.atomic_id).finish()
    }
}


impl<Event> AccountMarketFeed<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    pub fn new(stream: StreamKind<Event>) -> Self {
        Self {
            atomic_id: AtomicU64::new(0),
            data_stream: stream,
        }
    }

    pub fn increment_batch_id(&self) {
        self.atomic_id.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_batch_id(&self) -> u64 {
        self.atomic_id.load(Ordering::SeqCst)
    }
}


pub enum StreamKind<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    LiveFeed(LiveFeed<Event>),
    HistoricalFeed(HistoricalFeed<Event>),
}
