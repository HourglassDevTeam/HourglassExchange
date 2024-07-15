use crate::common_skeleton::datafeed::{Feed, MarketFeedDistributor};

/// 历史市场事件的 [`Feed`]。
/// 这是一个标准的批处理方案。
/// MarketFeed 接受一个泛型迭代器[Iter]，并允许用户按需逐个获取历史市场事件。
/// 这种方式适合处理离线数据或在内存中加载整个历史数据集的情况。
#[derive(Debug)]
pub struct MarketFeed<Iter, Event>
where
    Iter: Iterator<Item=Event>,
{
    pub market_iterator: Iter,
}

impl<Iter, Event> MarketFeedDistributor<Event> for MarketFeed<Iter, Event>
where
    Iter: Iterator<Item=Event>,
{
    fn fetch_next(&mut self) -> Feed<Event>
    {
        self.market_iterator.next().map_or(Feed::Finished, Feed::Next)
    }
}

impl<Iter, Event> MarketFeed<Iter, Event>
where
    Iter: Iterator<Item=Event>,
{
    pub fn new<IntoIter>(market_iterator: IntoIter) -> Self
    where
        IntoIter: IntoIterator<Item=Event, IntoIter=Iter>,
    {
        // 使用提供的事件迭代器创建一个新的 `MarketFeed` 实例
        Self { market_iterator: market_iterator.into_iter() }
    }
}
