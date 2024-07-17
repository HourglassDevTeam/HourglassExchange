use serde::{Deserialize, Serialize};

// 回测专用的模拟WebSocket交易信息，只有4个字段
use crate::simulated_exchange::load_from_clickhouse::queries_operations::TradeDataFromClickhouse;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
#[allow(non_snake_case)]
pub struct WsTrade
{
    #[serde(alias = "instrument_id", alias = "inst_id")]
    instId: String,
    #[serde(alias = "side")]
    side: String,
    #[serde(alias = "price")]
    px: String,
    #[serde(alias = "timestamp")]
    ts: String,
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
