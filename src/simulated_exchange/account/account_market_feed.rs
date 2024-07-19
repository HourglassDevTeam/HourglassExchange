use std::{
    collections::HashMap,
    fmt,
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use futures_core::Stream;
use futures_util::{FutureExt, StreamExt};

use crate::{
    common_skeleton::datafeed::{historical::HistoricalFeed, live::LiveFeed},
    simulated_exchange::account::account_market_feed::DataStream::{Historical, Live},
};

// 定义一个数据流别名，用于标识每个数据流。
pub type StreamID = String;

// 定义一个结构体，用于管理多个数据流。
pub struct AccountDataStreams<Event>
where Event: Clone + Send + Sync + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    pub streams: HashMap<StreamID, DataStream<Event>>, // 使用HashMap存储数据流，键为StreamID
}

// 为 AccountDataStreams 实现 Debug trait，方便调试。
impl<Event> Debug for AccountDataStreams<Event> where Event: Debug + Clone + Send + Sync + 'static + Ord/* 约束Event类型必须满足Debug, Clone, Send, Sync, 'static特性 */
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // 打印 AccountDataStreams 的调试信息，包括流的标识符。
        f.debug_struct("AccountMarketStreams").field("streams", &self.streams.keys().collect::<Vec<_>>()).finish()
    }
}

// 为 AccountDataStreams 实现创建和增减方法，用于管理数据流。
impl<Event> AccountDataStreams<Event> where Event: Clone + Send + Sync + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    // 创建一个新的 AccountDataStreams 实例。
    pub fn new() -> Self
    {
        Self { streams: HashMap::new() }
    }

    // 向AccountDataStreams中添加一个新的数据流。
    pub fn add_stream(&mut self, id: StreamID, stream: DataStream<Event>)
    {
        self.streams.insert(id, stream);
    }

    // 从AccountDataStreams中移除一个数据流。
    pub fn remove_stream(&mut self, id: StreamID)
    {
        self.streams.remove(&id);
    }

    // 将多个数据流合并为一个数据流。
    pub fn join(self) -> impl Stream<Item = Event> {
        let mut streams = self.streams.into_values().map(|s| s.into_stream().boxed()).collect::<Vec<_>>();
        let mut heap = BinaryHeap::new();

        // 初始化堆，将每个流的第一个事件放入堆中
        for stream in &mut streams {
            if let Some(event) = stream.next().now_or_never() {
                heap.push(Reverse(event));
            }
        }

        // 创建一个新的流，合并所有流的事件
        futures_util::stream::unfold((heap, streams), |(mut heap, mut streams)| async move {
            if let Some(Reverse(Some(event))) = heap.pop() {
                let next_stream = streams.iter_mut().find_map(|stream| {
                    let future = stream.next();
                    match future.now_or_never() {
                        Some(event) => Some((event, stream)),
                        None => None,
                    }
                });

                if let Some((Some(new_event), _stream)) = next_stream {
                    heap.push(Reverse(Some(new_event)));
                }

                return Some((event, (heap, streams)));
            }
            None
        })
    }
}

// 定义一个枚举，表示数据流的类型，可以是实时数据流或历史数据流。
pub enum DataStream<Event>
where Event: Clone + Send + Sync + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    Live(LiveFeed<Event>),             // 实时数据流
    Historical(HistoricalFeed<Event>), // 历史数据流
}


// 为DataStream实现Stream trait，使其可以作为异步流处理。
impl<Event> Stream for DataStream<Event> where Event: Clone + Send + Sync + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    type Item = Event;

    // 数据流中的元素类型为Event

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>
    {
        // 根据数据流的类型，调用相应的poll_next方法。
        match self.get_mut() {
            | Historical(feed) => Pin::new(&mut feed.stream).poll_next(cx),
            | Live(feed) => Pin::new(&mut feed.stream).poll_next(cx),
        }
    }
}


// 为DataStream实现额外的方法，方便获取下一个元素。
impl<Event> DataStream<Event>
where
    Event: Clone + Send + Sync + 'static + Ord, // 约束Event类型必须满足Clone, Send, Sync, 'static特性，并且可排序（Ord）
{
    // 将DataStream转换为一个Stream。
    pub fn into_stream(self) -> impl Stream<Item = Event> + 'static {
        match self {
            Live(feed) => feed.stream,
            Historical(feed) => feed.stream,
        }
    }}