use crate::{
    common::instrument::{kind::InstrumentKind, Instrument},
    sandbox::clickhouse_api::queries_operations::Row,
    Token,
};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MarketTrade
{
    pub exchange: String, // 注意：此字段及相关数据存储在数据库中，但截至2024年8月目前不适用。
    pub symbol: String,   // 注意：符号格式为 `base_quote` 代表永续合约，`base_quote_XXXX` 代表期货（取决于交易所的不同）。
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
}

/// 注意：当前适用于2024年8月。todo!() 需要更新。
impl MarketTrade
{
    pub fn parse_kind(&self) -> InstrumentKind
    {
        let parts: Vec<&str> = self.exchange.split('-').collect();

        if parts.len() == 2 {
            // 假设以 `perpetual` 结尾的为永续合约 FIXME 这个是非常不正确的临时处理方式。以后还是要用MarketEvent来包裹MarketTrade
            if parts[1].to_lowercase().ends_with("futures") {
                InstrumentKind::Perpetual
            }
            else {
                InstrumentKind::Spot
            }
        }
        else if parts.len() > 2 {
            // 假设以 `future` 结尾的为期货
            if parts.last().unwrap().to_lowercase().ends_with("future") {
                InstrumentKind::Future
            }
            else {
                InstrumentKind::Spot // 默认处理为现货，如果结尾不是 `future`
            }
        }
        else {
            InstrumentKind::Spot // 没有下划线，默认现货工具
        }
    }

    pub fn parse_instrument(&self) -> Option<Instrument>
    {
        let parts: Vec<&str> = self.symbol.split('_').collect();

        if parts.len() >= 2 {
            let base = Token(parts[0].to_string());
            let quote = Token(parts[1].to_string());

            // 根据symbol的格式来解析InstrumentKind
            let kind = self.parse_kind();

            Some(Instrument { base, quote, kind })
        }
        else {
            // 没有下划线，可能是现货或其他类型（需要进一步逻辑处理）
            None
        }
    }
}

impl MarketTrade
{
    pub fn parse_base(&self) -> Option<String>
    {
        let parts: Vec<&str> = self.symbol.split('_').collect();
        if parts.len() == 2 {
            Some(parts[0].to_string())
        }
        else {
            None
        }
    }

    pub fn parse_quote(&self) -> Option<String>
    {
        let parts: Vec<&str> = self.symbol.split('_').collect();
        if parts.len() == 2 {
            Some(parts[1].to_string())
        }
        else {
            None
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_parse_base()
    {
        let trade = MarketTrade { exchange: "example".to_string(),
                                  symbol: "BTC_USD".to_string(),
                                  side: "buy".to_string(),
                                  price: 10000.0,
                                  timestamp: 1625244000,
                                  amount: 1.0 };

        assert_eq!(trade.parse_base(), Some("BTC".to_string()));

        let trade = MarketTrade { exchange: "".to_string(),
                                  symbol: "ETH_BTC".to_string(),
                                  side: "".to_string(),
                                  price: 0.0,
                                  timestamp: 0,
                                  amount: 0.0 };

        assert_eq!(trade.parse_base(), Some("ETH".to_string()));

        let trade = MarketTrade { exchange: "".to_string(),
                                  symbol: "XRP".to_string(),
                                  side: "".to_string(),
                                  price: 0.0,
                                  timestamp: 0,
                                  amount: 0.0 };

        assert_eq!(trade.parse_base(), None);
    }

    #[test]
    fn test_parse_quote()
    {
        let trade = MarketTrade { exchange: "example".to_string(),
                                  symbol: "BTC_USD".to_string(),
                                  side: "buy".to_string(),
                                  price: 10000.0,
                                  timestamp: 1625244000,
                                  amount: 1.0 };

        assert_eq!(trade.parse_quote(), Some("USD".to_string()));

        let trade = MarketTrade { exchange: "".to_string(),
                                  symbol: "ETH_BTC".to_string(),
                                  side: "".to_string(),
                                  price: 0.0,
                                  timestamp: 0,
                                  amount: 0.0 };

        assert_eq!(trade.parse_quote(), Some("BTC".to_string()));

        let trade = MarketTrade { exchange: "".to_string(),
                                  symbol: "XRP".to_string(),
                                  side: "".to_string(),
                                  price: 0.0,
                                  timestamp: 0,
                                  amount: 0.0 };

        assert_eq!(trade.parse_quote(), None);
    }
}
