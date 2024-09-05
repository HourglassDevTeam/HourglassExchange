use crate::common::datafeed::market_event::MarketEvent;
/// NOTE code below is to be merged later
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        instrument::{
            kind::{InstrumentKind, InstrumentKind::Perpetual},
            Instrument,
        },
        token::Token,
    },
    sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade,
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
    amount: f64,
}

// NOTE 这是按照Okex交易所API数据类型构建的 WebsocketTrade 数据结构，回测选用。
impl MarketEvent<WsTrade>
{
    pub fn from_ws_trade(ws_trade: WsTrade, base: String, quote: String, instrument: InstrumentKind) -> Self
    {
        let exchange_time = ws_trade.ts.parse::<i64>().unwrap_or(0);
        let received_time = ws_trade.ts.parse::<i64>().unwrap_or(0); // NOTE 注意这是不对的 应该加上一个标准化的随机延迟。

        let instrument = Instrument { base: Token::from(base),
                                      quote: Token::from(quote),
                                      kind: instrument };

        MarketEvent { exchange_ts: exchange_time,
                      received_ts: received_time,
                      exchange: Exchange::SandBox,

                      instrument,
                      kind: ws_trade }
    }
}

// NOTE 这是按照Clickhouse中存储的数据类型构建的 WebsocketTrade 数据结构，回测选用。
impl MarketEvent<MarketTrade>
{
    pub fn from_swap_trade_clickhouse(trade: MarketTrade, base: String, quote: String) -> Self
    {
        let exchange_time = trade.timestamp;
        let received_time = trade.timestamp; // NOTE 注意这是不对的 应该加上一个标准化的随机延迟。

        let instrument = Instrument { base: Token::from(base),
                                      quote: Token::from(quote),
                                      kind: Perpetual };

        MarketEvent { exchange_ts: exchange_time,
                      received_ts: received_time,
                      exchange: Exchange::SandBox,
                      instrument,
                      kind: trade }
    }
}

// 从 TradeDataFromClickhouse 到 WsTrade 的转换实现
impl From<MarketTrade> for WsTrade
{
    fn from(trade: MarketTrade) -> Self
    {
        WsTrade { instId: trade.symbol,
                  side: trade.side,
                  px: trade.price.to_string(),
                  ts: trade.timestamp.to_string(),
                  amount: trade.amount }
    }
}

pub fn parse_base_and_quote(symbol: &str) -> (String, String)
{
    // 定义一个包含常见报价货币的数组
    let quote_assets = ["USDT", "USTC", "USDC", "USD", "UST", "DAI", "FDUSD", "BTC", "ETH", "EURT"];
    // 遍历所有已知的报价货币
    for &quote in &quote_assets {
        // 尝试去除报价货币的后缀
        if let Some(base) = symbol.strip_suffix(quote) {
            // 返回基础货币和报价货币
            return (base.to_string(), quote.to_string());
        }
    }
    // 如果没有匹配的报价货币，返回原始符号作为基础货币，报价货币为空字符串
    (symbol.to_string(), String::new())
}

#[allow(dead_code)]
impl WsTrade
{
    pub(crate) fn from_ref(trade: &MarketTrade) -> Self
    {
        WsTrade { // 这里假设 WsTrade 结构体字段和 TradeDataFromClickhouse 结构体字段对应
                  instId: trade.symbol.clone(),
                  side: trade.side.clone(),
                  px: trade.price.to_string(),
                  ts: trade.timestamp.to_string(),
                  amount: trade.amount }
    }
}
