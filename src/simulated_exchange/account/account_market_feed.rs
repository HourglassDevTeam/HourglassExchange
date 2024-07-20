use std::{
    collections::HashMap,
    fmt,
    fmt::Debug

    ,
};

use cerebro_data::exchange::Connector;
use cerebro_data::Identifier;
use cerebro_data::streams::builder::SocketError;
use cerebro_data::subscription::SubKind;

use crate::common_skeleton::{
    datafeed::{historical::HistoricalFeed, live::LiveFeed},
    instrument::Instrument,
};

// 定义一个数据流别名，用于标识每个数据流。
pub type StreamID = String;

// 定义一个结构体，用于管理多个数据流。
pub struct AccountDataStreams<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    pub streams: HashMap<StreamID, DataStreams<Event>>, // 使用HashMap存储数据流，键为StreamID
}

// 为 AccountDataStreams 实现 Debug trait，方便调试。
impl<Event> Debug for AccountDataStreams<Event>
    where Event: Debug + Clone + Send + Sync + Debug + 'static + Ord /* 约束Event类型必须满足Debug, Clone, Send, Sync, 'static特性 */
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // 打印 AccountDataStreams 的调试信息，包括流的标识符。
        f.debug_struct("AccountMarketStreams").field("streams", &self.streams.keys().collect::<Vec<_>>()).finish()
    }
}



// NOTE this is foreign to this module
#[derive(Debug)]
pub struct Subscription<Exchange, Kind>
{
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: Kind,
}

// 为 AccountDataStreams 实现创建和增减数据流的方法，用于管理数据流。
impl<Event> AccountDataStreams<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    // 创建一个新的 AccountDataStreams 实例。
    pub fn new() -> Self
    {
        Self { streams: HashMap::new() }
    }

    // 向AccountDataStreams中添加一个新的数据流。
    pub fn add_stream(&mut self, id: StreamID, stream: DataStreams<Event>)
    {
        self.streams.insert(id, stream);
    }

    // 从AccountDataStreams中移除一个数据流。
    pub fn remove_stream(&mut self, id: StreamID)
    {
        self.streams.remove(&id);
    }
}

// 定义一个枚举，表示数据流的类型，可以是实时数据流或历史数据流。
pub enum DataStreams<Event>
    where Event: Clone + Send + Sync + Debug + 'static + Ord /* 约束Event类型必须满足Clone, Send, Sync, 'static特性 */
{
    Live(LiveFeed<Event>),             // 实时数据流
    Historical(HistoricalFeed<Event>), // 历史数据流
}


