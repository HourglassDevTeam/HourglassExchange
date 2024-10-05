use crate::{
    common::{
        instrument::{kind::InstrumentKind, Instrument},
        stable_token::StableToken,
    },
    hourglass::clickhouse_api::queries_operations::Row,
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
            // 假设以 `futures` 结尾的为期货
            if parts.last().unwrap().to_lowercase().ends_with("futures") {
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
        // 遍历所有的 `StableToken` 变种，并检查 symbol 是否以该稳定币结尾
        let possible_quote = [StableToken::Tether,
                              StableToken::USD,
                              StableToken::BinanceUSD,
                              StableToken::Dai,
                              StableToken::PaxosStandard,
                              StableToken::TrueUSD,
                              StableToken::GeminiDollar,
                              StableToken::TerraUSD,
                              StableToken::Frax,
                              StableToken::NeutrinoUSD].iter()
                                                       .find_map(|stable_token| {
                                                           let token_quote = stable_token.to_token();
                                                           if self.symbol.ends_with(token_quote.as_ref()) {
                                                               Some(token_quote)
                                                           }
                                                           else {
                                                               None
                                                           }
                                                       });

        if let Some(quote_token) = possible_quote {
            let base = self.symbol.trim_end_matches(quote_token.as_ref()).to_string();
            let token_base = Token::new(&base);

            // 根据symbol的格式来解析InstrumentKind
            let kind = self.parse_kind();

            Some(Instrument { base: token_base,
                              quote: quote_token,
                              kind })
        }
        else {
            println!("Unrecognized symbol: {}", self.symbol); // 调试输出
            None // 无法解析出base和quote
        }
    }
}
impl MarketTrade
{
    pub fn parse_base(&self) -> Option<String>
    {
        // 遍历所有的 `StableToken` 变种，并检查 symbol 是否以该稳定币结尾
        let possible_quote = [StableToken::Tether,
                              StableToken::USD,
                              StableToken::BinanceUSD,
                              StableToken::Dai,
                              StableToken::PaxosStandard,
                              StableToken::TrueUSD,
                              StableToken::GeminiDollar,
                              StableToken::TerraUSD,
                              StableToken::Frax,
                              StableToken::NeutrinoUSD].iter()
                                                       .find_map(|stable_token| {
                                                           let token_quote = stable_token.to_token();
                                                           if self.symbol.ends_with(token_quote.as_ref()) {
                                                               Some(token_quote)
                                                           }
                                                           else {
                                                               None
                                                           }
                                                       });

        if let Some(quote_token) = possible_quote {
            // 从 symbol 中去掉 quote 部分，剩下的就是 base
            Some(self.symbol.trim_end_matches(quote_token.as_ref()).to_string())
        }
        else {
            None
        }
    }

    pub fn parse_quote(&self) -> Option<String>
    {
        // 遍历所有的 `StableToken` 变种，并检查 symbol 是否以该稳定币结尾
        let possible_quote = [StableToken::Tether,
                              StableToken::USD,
                              StableToken::BinanceUSD,
                              StableToken::Dai,
                              StableToken::PaxosStandard,
                              StableToken::TrueUSD,
                              StableToken::GeminiDollar,
                              StableToken::TerraUSD,
                              StableToken::Frax,
                              StableToken::NeutrinoUSD].iter()
                                                       .find_map(|stable_token| {
                                                           let token_quote = stable_token.to_token();
                                                           if self.symbol.ends_with(token_quote.as_ref()) {
                                                               Some(token_quote.as_ref().to_string())
                                                           }
                                                           else {
                                                               None
                                                           }
                                                       });

        possible_quote
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_parse_instrument()
    {
        let trade = MarketTrade { exchange: "example".to_string(),
                                  symbol: "BTCUSDT".to_string(),
                                  side: "buy".to_string(),
                                  price: 10000.0,
                                  timestamp: 1625244000,
                                  amount: 1.0 };

        let instrument = trade.parse_instrument().unwrap();
        assert_eq!(instrument.base.as_ref(), "BTC");
        assert_eq!(instrument.quote.as_ref(), "USDT");

        let trade = MarketTrade { exchange: "example".to_string(),
                                  symbol: "ETHUSDT".to_string(),
                                  side: "buy".to_string(),
                                  price: 2000.0,
                                  timestamp: 1625245000,
                                  amount: 1.0 };

        let instrument = trade.parse_instrument().unwrap();
        assert_eq!(instrument.base.as_ref(), "ETH");
        assert_eq!(instrument.quote.as_ref(), "USDT");

        let trade = MarketTrade { exchange: "example".to_string(),
                                  symbol: "XRP".to_string(),
                                  side: "buy".to_string(),
                                  price: 0.5,
                                  timestamp: 1625246000,
                                  amount: 1.0 };

        assert!(trade.parse_instrument().is_none());
    }
}
