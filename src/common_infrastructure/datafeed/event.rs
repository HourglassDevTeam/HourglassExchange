use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{
    common_infrastructure::{
        instrument::Instrument,
        order::{FullyFill, Order, PartialFill},
        trade::ClientTrade,
        Side,
    },
    sandbox::{clickhouse_api::datatype::clickhouse_trade_data::ClickhousePublicTrade, ws_trade::WsTrade},
    ExchangeVariant,
};

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
impl From<Order<FullyFill>> for MarketEvent<ClickhousePublicTrade>
{
    fn from(order: Order<FullyFill>) -> Self
    {
        let clickhouse_trade = ClickhousePublicTrade { basequote: format!("{}/{}", order.instrument.base, order.instrument.quote),
                                                 side: match order.side {
                                                     | Side::Buy => "buy".to_string(),
                                                     | Side::Sell => "sell".to_string(),
                                                 },
                                                 price: order.state.price,
                                                 timestamp: order.state.id.0.parse().unwrap_or_default(),
                                                 amount: order.state.size };

        MarketEvent { exchange_time: order.state.id.0.parse().unwrap_or_default(),
                      received_time: order.client_ts,
                      exchange: order.exchange,
                      instrument: order.instrument,
                      kind: clickhouse_trade }
    }
}

// 为 Order<PartialFill> 实现 From trait
impl From<Order<PartialFill>> for MarketEvent<ClickhousePublicTrade>
{
    fn from(order: Order<PartialFill>) -> Self
    {
        let clickhouse_trade = ClickhousePublicTrade { basequote: format!("{}/{}", order.instrument.base, order.instrument.quote),
                                                 side: match order.side {
                                                     | Side::Buy => "buy".to_string(),
                                                     | Side::Sell => "sell".to_string(),
                                                 },
                                                 price: order.state.price,
                                                 timestamp: order.state.id.0.parse().unwrap_or_default(),
                                                 amount: order.state.size };

        MarketEvent { exchange_time: order.state.id.0.parse().unwrap_or_default(),
                      received_time: order.client_ts,
                      exchange: order.exchange,
                      instrument: order.instrument,
                      kind: clickhouse_trade }
    }
}
