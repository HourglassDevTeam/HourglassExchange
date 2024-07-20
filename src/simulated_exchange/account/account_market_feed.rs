use std::{collections::HashMap, fmt, fmt::Debug};

use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;

use crate::common_skeleton::instrument::Instrument;

// 定义一个数据流别名，用于标识每个数据流。
pub type StreamID = String;

// 定义一个结构体，用于管理多个数据流。
pub struct AccountDataStreams<Event>
    where Event: Debug + Clone + Send + Sync + 'static + Ord
{
    pub streams: HashMap<StreamID, UnboundedReceiver<Event>>, // 使用HashMap存储数据流，键为StreamID
}


// NOTE this is foreign to this module
pub struct Subscription<Exchange, Kind>
{
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: Kind,
}

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
}


// 为 AccountDataStreams 实现 Debug trait，方便调试。
impl<Event> Debug for AccountDataStreams<Event> where Event: Debug + Clone + Send + Sync + 'static + Ord
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // 打印 AccountDataStreams 的调试信息，包括流的标识符。
        let stream_keys: Vec<_> = self.streams.keys().collect();
        f.debug_struct("AccountDataStreams")
            .field("streams", &stream_keys)
            .finish()
    }
}