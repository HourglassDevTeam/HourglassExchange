use serde::{Deserialize, Serialize};

use crate::{
    common_skeleton::{instrument::Instrument, trade::Trade},
    Exchange,
    simulated_exchange::ws_trade::WsTrade,
};

// 定义一个泛型结构体 MarketEvent，包含各种交易市场事件信息
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<T>
{
    pub exchange_time: i64,     // 交易所时间戳
    pub received_time: i64,     // 接收到数据的时间戳
    pub exchange: Exchange,     // 交易所信息
    pub instrument: Instrument, // 交易工具信息
    pub kind: T,                // 事件的具体类型
}

// 定义一个枚举类型 DataKind，用于表示不同种类的数据
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum DataKind
{
    WsTrade(WsTrade), // WebSocket 交易数据
    Trade(Trade),
    /* 普通交易数据
                      * OrderBook25(OrderBook25), // 订单簿数据
                      * Candle(Candle),           // 蜡烛图数据
                      * Liquidation(Liquidation), // 清算数据 */
}

// 为 MarketEvent<Trade> 实现转换为 MarketEvent<DataKind> 的方法
impl From<MarketEvent<Trade>> for MarketEvent<DataKind>
{
    fn from(event: MarketEvent<Trade>) -> Self
    {
        // 将 Trade 类型的 MarketEvent 转换为 DataKind::Trade 类型的 MarketEvent
        Self {
            exchange_time: event.exchange_time,
            received_time: event.received_time,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Trade(event.kind),
        }
    }
}

// 为 MarketEvent<WsTrade> 实现转换为 MarketEvent<DataKind> 的方法
impl From<MarketEvent<WsTrade>> for MarketEvent<DataKind>
{
    fn from(event: MarketEvent<WsTrade>) -> Self
    {
        // 将 WsTrade 类型的 MarketEvent 转换为 DataKind::WsTrade 类型的 MarketEvent
        Self {
            exchange_time: event.exchange_time,
            received_time: event.received_time,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::WsTrade(event.kind),
        }
    }
}
