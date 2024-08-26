use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{
        instrument::Instrument,
        order::{FullyFill, Order, PartialFill},
        trade::ClientTrade,
    },
    sandbox::{clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade, ws_trade::WsTrade},
    ExchangeVariant,
};
use crate::common_infrastructure::trade::ClientTradeId;

// 定义一个泛型结构体 MarketEvent，包含各种交易市场事件信息
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct MarketEvent<Data>
{
    pub exchange_time: i64,        // 交易所时间戳
    pub received_time: i64,        // 接收到数据的时间戳
    pub exchange: ExchangeVariant, // 交易所信息
    pub instrument: Instrument,    // 交易工具信息
    pub kind: Data,                // 事件的具体类型
}

// 定义一个枚举类型 DataKind，用于表示不同种类的数据
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum DataKind
{
    WsTrade(WsTrade), // WebSocket 交易数据
    Trade(ClientTrade),
    ClickhousePublicTrade(ClickhousePublicTrade),
    // OrderBook25(OrderBook25), // 订单簿数据
    // Candle(Candle),           // 蜡烛图数据
    // Liquidation(Liquidation), // 清算数据
}

// 为 MarketEvent<Trade> 实现转换为 MarketEvent<DataKind> 的方法
impl From<MarketEvent<ClientTrade>> for MarketEvent<DataKind>
{
    fn from(event: MarketEvent<ClientTrade>) -> Self
    {
        // 将 Trade 类型的 MarketEvent 转换为 DataKind::Trade 类型的 MarketEvent
        Self { exchange_time: event.exchange_time,
               received_time: event.received_time,
               exchange: event.exchange,
               instrument: event.instrument,
               kind: DataKind::Trade(event.kind) }
    }
}

// 为 MarketEvent<WsTrade> 实现转换为 MarketEvent<DataKind> 的方法
impl From<MarketEvent<WsTrade>> for MarketEvent<DataKind>
{
    fn from(event: MarketEvent<WsTrade>) -> Self
    {
        // 将 WsTrade 类型的 MarketEvent 转换为 DataKind::WsTrade 类型的 MarketEvent
        Self { exchange_time: event.exchange_time,
               received_time: event.received_time,
               exchange: event.exchange,
               instrument: event.instrument,
               kind: DataKind::WsTrade(event.kind) }
    }
}
// 为 Order<FullyFill> 实现 From trait
impl From<Order<FullyFill>> for MarketEvent<ClientTrade> {
    fn from(order: Order<FullyFill>) -> Self {
        let client_trade = ClientTrade {
            id: ClientTradeId(order.state.id.0.parse().unwrap_or_default()),
            instrument: order.instrument.clone(),
            side: order.side,
            price: order.state.price,
            size: order.state.size,
            count: 1, // 假设 fully filled 的订单只有一个 trade
            fees: 0.0, // 根据你的逻辑调整 fees 计算
        };

        MarketEvent {
            exchange_time: order.state.id.0.parse().unwrap_or_default(),
            received_time: order.client_ts,
            exchange: order.exchange,
            instrument: order.instrument.clone(),
            kind: client_trade,
        }
    }
}

// 为 Order<PartialFill> 实现 From trait
impl From<Order<PartialFill>> for MarketEvent<ClientTrade> {
    fn from(order: Order<PartialFill>) -> Self {
        let client_trade = ClientTrade {
            id: ClientTradeId(order.state.id.0.parse().unwrap_or_default()),
            instrument: order.instrument.clone(),
            side: order.side,
            price: order.state.price,
            size: order.state.size,
            count: 1, // 假设部分成交的订单也只有一个 trade
            fees: 0.0, // 根据你的逻辑调整 fees 计算
        };

        MarketEvent {
            exchange_time: order.state.id.0.parse().unwrap_or_default(),
            received_time: order.client_ts,
            exchange: order.exchange,
            instrument: order.instrument.clone(),
            kind: client_trade,
        }
    }
}
