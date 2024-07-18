/// NOTE code below is to be merged later
use serde::{Deserialize, Serialize};

use crate::{
    common_skeleton::{
        datafeed::event::MarketEvent,
        instrument::{kind::InstrumentKind, Instrument},
        token::Token,
    },
    simulated_exchange::load_from_clickhouse::queries_operations::TradeDataFromClickhouse,
    Exchange,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
#[allow(non_snake_case)]
pub struct WsTrade
{
    #[serde(alias = "instrument_id", alias = "inst_id")]
    instId: String, // NOTE nomenclature of instrument ID ?
    #[serde(alias = "side")]
    side: String,
    #[serde(alias = "price")]
    px: String,
    #[serde(alias = "timestamp")]
    ts: String,
}

// NOTE 按照API构建的 WebsocketTrade 数据结构，回测选用。
impl MarketEvent<WsTrade>
{
    pub fn from_ws_trade(ws_trade: WsTrade, base: String, quote: String, exchange: Exchange) -> Self
    {
        let exchange_time = ws_trade.ts.parse::<i64>().unwrap_or(0);
        let received_time = ws_trade.ts.parse::<i64>().unwrap_or(0);

        let instrument = Instrument { base: Token::from(base),
                                      quote: Token::from(quote),
                                      kind: InstrumentKind::Spot };

        MarketEvent { exchange_time,
                      received_time,
                      exchange,
                      instrument,
                      kind: ws_trade }
    }
}

// NOTE 回测专用
impl MarketEvent<TradeDataFromClickhouse>
{
    pub fn from_trade_clickhouse(trade: TradeDataFromClickhouse, base: String, quote: String, exchange: Exchange) -> Self
    {
        let exchange_time = trade.timestamp;
        let received_time = trade.timestamp;

        let instrument = Instrument { base: Token::from(base),
                                      quote: Token::from(quote),
                                      kind: InstrumentKind::Spot };

        MarketEvent { exchange_time,
                      received_time,
                      exchange,
                      instrument,
                      kind: trade }
    }
}

// 从 TradeDataFromClickhouse 到 WsTrade 的转换实现
impl From<TradeDataFromClickhouse> for WsTrade
{
    fn from(trade: TradeDataFromClickhouse) -> Self
    {
        WsTrade { instId: trade.symbol,
                  side: trade.side,
                  px: trade.price.to_string(),
                  ts: trade.timestamp.to_string() }
    }
}

impl WsTrade
{
    pub(crate) fn from_ref(data: &TradeDataFromClickhouse) -> Self
    {
        WsTrade { // 这里假设 WsTrade 结构体字段和 TradeDataFromClickhouse 结构体字段对应
                  instId: data.symbol.clone(),
                  side: data.side.clone(),
                  px: data.price.to_string(),
                  ts: data.timestamp.to_string() }
    }
}
