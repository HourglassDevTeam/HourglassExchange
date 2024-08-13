use std::{collections::HashMap, fmt, fmt::Debug};

use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;

// 定义一个数据流别名，用于标识每个数据流。
pub type StreamID = String;

// 定义一个结构体，用于管理多个数据流。
pub struct AccountDataStreams<Event>
    where Event: Debug + Clone + Send + Sync + 'static + Ord
{
    pub streams: HashMap<StreamID, UnboundedReceiver<Event>>, // 使用HashMap存储数据流，键为StreamID
}

// // NOTE this is foreign to this module
// pub struct Subscription<Exchange, Kind>
// {
//     pub exchange: Exchange,
//     pub instrument: Instrument,
//     pub kind: Kind,
// }

// 为 AccountDataStreams 实现创建和增减数据流的方法，用于管理数据流。
impl<Event> AccountDataStreams<Event> where Event: Debug + Clone + Send + Sync + 'static + Ord
{
    // 创建一个新的 AccountDataStreams 实例。
    pub fn new() -> Self
    {
        Self { streams: HashMap::new() }
    }

    // 向AccountDataStreams中添加一个新的数据流。
    pub fn add_stream(&mut self, id: StreamID, receiver: UnboundedReceiver<Event>)
    {
        self.streams.insert(id, receiver);
    }

    // 从AccountDataStreams中移除一个数据流。
    pub fn remove_stream(&mut self, id: StreamID)
    {
        self.streams.remove(&id);
    }

    // 将所有数据流合并到一个新的接收器中，并按时间戳排序。
    pub async fn join(self) -> UnboundedReceiver<Event>
        where Event: Send + 'static
    {
        let mut joined_rx = self.merge_streams().await;

        // 监听合并后的接收器，按时间戳排序后再发送到新的接收器中
        let (sorted_tx, sorted_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut buffer = Vec::new();

            while let Some(event) = joined_rx.recv().await {
                buffer.push(event);
                buffer.sort(); // 按时间戳排序
                for e in buffer.drain(..) {
                    let _ = sorted_tx.send(e);
                }
            }
        });

        sorted_rx
    }

    // 将所有数据流合并到一个新的接收器中，不进行排序。
    pub async fn join_without_sort(self) -> UnboundedReceiver<Event>
        where Event: Send + 'static
    {
        self.merge_streams().await
    }

    // 合并所有数据流到一个新的接收器中。
    async fn merge_streams(self) -> UnboundedReceiver<Event>
        where Event: Send + 'static
    {
        let (joined_tx, joined_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let joined_tx = joined_tx.clone();

            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = joined_tx.send(event);
                }
            });
        }

        joined_rx
    }
}

// 为 AccountDataStreams 实现 Debug trait，方便调试。
impl<Event> Debug for AccountDataStreams<Event> where Event: Debug + Clone + Send + Sync + 'static + Ord
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // 打印 AccountDataStreams 的调试信息，包括流的标识符。
        let stream_keys: Vec<_> = self.streams.keys().collect();
        f.debug_struct("AccountDataStreams").field("streams", &stream_keys).finish()
    }
}
