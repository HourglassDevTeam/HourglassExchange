use std::{
    collections::HashMap,
    fmt,
    fmt::Debug

    ,
};
use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;

use crate::{
    common_skeleton::{
        datafeed::{historical::HistoricalFeed, live::LiveFeed},
        instrument::Instrument,
    },
    data_subscriber::{
        connector::Connector,
        socket_error::SocketError,
        subscriber::{Identifier, SubKind},
    },
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
impl<Event> AccountDataStreams<Event> where Event: Clone + Send + Sync + Debug + 'static + Ord
{
    // 添加一个新的方法用于添加WebSocket实时数据流
    pub async fn add_websocket_stream<Exchange, Kind>(&mut self, id: StreamID, subscriptions: &[Subscription<Exchange, Kind>]) -> Result<(), SocketError>
        where Exchange: Connector + Send + Sync,
              Kind: SubKind + Send + Sync,
              Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>
    {
        let stream = DataStreams::from_websocket::<Exchange, Kind>(subscriptions).await?;
        self.add_stream(id, stream);
        Ok(())
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


impl<T> DataStreams<T> {
    /// Construct a [`StreamBuilder`] for configuring new
    /// [`MarketEvent<SubKind::Event>`](crate::event::MarketEvent) [`DataStreams`].

    pub fn builder<Kind>() -> StreamBuilder<Kind>
                           where
                               Kind: SubKind,
    {
        StreamBuilder::<Kind>::new()
    }

    /// Construct a [`MultiStreamBuilder`] for configuring new
    /// [`MarketEvent<T>`](crate::event::MarketEvent) [`DataStreams`].

    pub fn builder_multi() -> MultiStreamBuilder<T> {
        MultiStreamBuilder::<T>::new()
    }


    pub fn select(&mut self, exchange: ExchangeId) -> Option<UnboundedReceiver<T>> {
        self.streams.remove(&exchange)
    }


    pub async fn join(self) -> UnboundedReceiver<T>
                      where
                          T: Send + 'static,
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

    /// Join all exchange [`UnboundedReceiver`] streams into a unified [`StreamMap`].

    pub async fn join_map(self) -> StreamMap<ExchangeId, UnboundedReceiverStream<T>> {
        self.streams.into_iter().fold(StreamMap::new(), |mut map, (exchange, rx)| {
            map.insert(exchange, UnboundedReceiverStream::new(rx));

            map
        })
    }
}
