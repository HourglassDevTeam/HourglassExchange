use crate::common_skeleton::datafeed::{historical::MarketFeed, MarketFeedDistributor};
use std::sync::atomic::{AtomicU64, Ordering};

// 鉴于Data的种类可能会很多，规避避开enum的开销和维护成本，使用泛型来定义AccountFeedData类型。
#[derive(Debug)]
pub struct AccountMarketFeed<Iter, Event>
    where Iter: Iterator<Item = Event> + Clone,
          MarketFeed<Iter, Event>: MarketFeedDistributor<Event>
{
    // NOTE 每次循环载入数据后batch_id都会加1
    pub atomic_id: AtomicU64,
    pub data: MarketFeed<Iter, Event>,
}

impl<Iter, Event> AccountMarketFeed<Iter, Event>
    where Iter: Iterator<Item = Event> + Clone,
          MarketFeed<Iter, Event>: MarketFeedDistributor<Event>
{
    pub fn new(market_feed: MarketFeed<Iter, Event>) -> Self
    {
        Self { atomic_id: AtomicU64::new(0),
               data: market_feed }
    }

    pub fn increment_batch_id(&self)
    {
        self.atomic_id.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_batch_id(&self) -> u64
    {
        self.atomic_id.load(Ordering::SeqCst)
    }
}
