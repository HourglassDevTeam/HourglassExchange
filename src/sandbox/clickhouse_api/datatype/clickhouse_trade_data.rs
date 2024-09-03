use crate::sandbox::clickhouse_api::queries_operations::Row;
use serde::{Deserialize, Serialize};
use crate::common::instrument::kind::InstrumentKind;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MarketTrade {
    pub exchange: String, // 注意：此字段及相关数据存储在数据库中，但截至2024年8月目前不适用。
    pub symbol: String,   // 注意：符号格式为 `base_quote` 代表永续合约，`base_quote_XXXX` 代表期货（取决于交易所的不同）。
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
}


/// 注意：当前适用于2024年8月。todo!() 需要更新。
impl MarketTrade {
    pub fn parse_kind(&self) -> InstrumentKind {
        // 检查符号是否包含下划线
        if self.symbol.contains('_') {
            // 检查符号中是否包含多个下划线
            if self.symbol.matches('_').count() > 1 {
                InstrumentKind::Future
            } else {
                // 如果符号包含一个下划线，假设为永续合约
                InstrumentKind::Perpetual
            }
        } else {
            // 处理没有下划线的符号
            // 这里可以添加更多逻辑来区分现货、加密期权等
            // 目前，如果没有下划线，我们假设为现货工具
            todo!()
        }
    }
}



impl MarketTrade {
    pub fn parse_base(&self) -> Option<String> {
        let parts: Vec<&str> = self.symbol.split('_').collect();
        if parts.len() == 2 {
            Some(parts[0].to_string())
        } else {
            None
        }
    }

    pub fn parse_quote(&self) -> Option<String> {
        let parts: Vec<&str> = self.symbol.split('_').collect();
        if parts.len() == 2 {
            Some(parts[1].to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base() {
        let trade = MarketTrade {
            exchange: "example".to_string(),
            symbol: "BTC_USD".to_string(),
            side: "buy".to_string(),
            price: 10000.0,
            timestamp: 1625244000,
            amount: 1.0,
        };

        assert_eq!(trade.parse_base(), Some("BTC".to_string()));

        let trade = MarketTrade {
            exchange: "".to_string(),
            symbol: "ETH_BTC".to_string(),
            side: "".to_string(),
            price: 0.0,
            timestamp: 0,
            amount: 0.0,
        };

        assert_eq!(trade.parse_base(), Some("ETH".to_string()));

        let trade = MarketTrade {
            exchange: "".to_string(),
            symbol: "XRP".to_string(),
            side: "".to_string(),
            price: 0.0,
            timestamp: 0,
            amount: 0.0,
        };

        assert_eq!(trade.parse_base(), None);
    }

    #[test]
    fn test_parse_quote() {
        let trade = MarketTrade {
            exchange: "example".to_string(),
            symbol: "BTC_USD".to_string(),
            side: "buy".to_string(),
            price: 10000.0,
            timestamp: 1625244000,
            amount: 1.0,
        };

        assert_eq!(trade.parse_quote(), Some("USD".to_string()));

        let trade = MarketTrade {
            exchange: "".to_string(),
            symbol: "ETH_BTC".to_string(),
            side: "".to_string(),
            price: 0.0,
            timestamp: 0,
            amount: 0.0,
        };

        assert_eq!(trade.parse_quote(), Some("BTC".to_string()));

        let trade = MarketTrade {
            exchange: "".to_string(),
            symbol: "XRP".to_string(),
            side: "".to_string(),
            price: 0.0,
            timestamp: 0,
            amount: 0.0,
        };

        assert_eq!(trade.parse_quote(), None);
    }
}
