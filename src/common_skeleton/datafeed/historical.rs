use crate::common_skeleton::datafeed::{FeedStatus, MarketFeedDistributor};

// 从 common_skeleton 模块中引入 Feed 和 MarketFeedDistributor
// 这两个结构体或特性用于处理市场数据

/// 历史市场事件的 [`FeedStatus`]。
/// MarketFeed 接受一个泛型迭代器[Iter]，并允许用户按需逐个获取历史市场事件。
/// 这种方式适合处理离线数据或在内存中加载整个历史数据集的情况。
#[derive(Debug)]
pub struct HistoricalFeed<Event>
where
    Iter: Iterator<Item=Event>,
{
    // 定义一个结构体 HistoricalFeed，带有两个泛型参数 Iter 和 Event
    // Iter 必须是一个迭代器，且其迭代项是 Event 类型的元素
    pub market_iterator: Iter, // 这个字段是一个迭代器，用于按需获取历史市场事件
}

impl<Event> MarketFeedDistributor<Event> for HistoricalFeed<Event>
where
    Iter: Iterator<Item=Event>,
{
    // 为 HistoricalFeed 实现 MarketFeedDistributor 特性
    fn fetch_next(&mut self) -> FeedStatus<Event>
    {
        // fetch_next 方法尝试从迭代器中获取下一个事件
        // 如果有事件，返回 Feed::Next(event)
        // 如果没有事件，返回 Feed::Finished
        self.market_iterator.next().map_or(FeedStatus::Finished, FeedStatus::Next)
    }
}

// HistoricalFeed生成的办法，新建一个接受Event泛型的历史市场事件迭代器
impl<Event> HistoricalFeed<Event>
where
    Iter: Iterator<Item=Event>,
{
    // 为 HistoricalFeed 实现一个新的关联函数 initiate
    // 这个函数用于从一个实现了 IntoIterator 特性的类型创建 HistoricalFeed
    pub fn initiate<IntoIter>(market_iterator: IntoIter) -> Self
    where
        IntoIter: IntoIterator<Item=Event, IntoIter=Iter>,
    {
        // 使用提供的迭代器创建一个新的 HistoricalFeed 实例
        Self { market_iterator: market_iterator.into_iter() }
    }
}
