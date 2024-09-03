use crate::sandbox::clickhouse_api::queries_operations::Row;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct MarketTrade
{
    pub exchange: String, // NOTE this field and relevant data is stored in database. but currently not applicable as of August 2024.
    pub symbol: String,   // NOTE : symbol = basequote
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
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
