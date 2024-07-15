use uuid::Uuid;

use crate::common_skeleton::datafeed::{historical::MarketFeed, MarketFeedDistributor};

// 鉴于Data的种类可能会很多，规避避开enum的开销和维护成本，使用泛型来定义AccountFeedData类型。
#[derive(Debug)]
pub struct AccountMarketFeed<Iter, Event>
where
    Iter: Iterator<Item=Event> + Clone,
    MarketFeed<Iter, Event>: MarketFeedDistributor<Event>,
{
    pub batch_id: Uuid,
    pub data: MarketFeed<Iter, Event>,
}
