use crate::{
    common::Side,
    hourglass::clickhouse_api::{datatype::clickhouse_trade_data::MarketTrade, queries_operations::Row},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct SingleLevelOrderBook
{
    pub latest_bid: f64,
    pub latest_ask: f64,
    pub latest_price: f64, // 记录最新的交易价格
}

pub trait OrderBookUpdater
{
    fn update_from_trade(&mut self, market_trade: &MarketTrade);
}

impl OrderBookUpdater for SingleLevelOrderBook
{
    // NOTE 这里要做一个特殊的机制。为了不让价格未零。当第一条trade进来以后，除了更新本side以外，要把另外一个side的价格也更新成同样的价格。之后再正常更新。
    fn update_from_trade(&mut self, market_trade: &MarketTrade) {
        match Side::from_str(&market_trade.side) {
            Ok(Side::Buy) => {
                // 如果是买单，更新最新的买方价格
                self.latest_bid = market_trade.price;
                // 如果卖方价格为0，初始化为相同的价格
                if self.latest_ask == 0.0 {
                    self.latest_ask = market_trade.price;
                }
            }
            Ok(Side::Sell) => {
                // 如果是卖单，更新最新的卖方价格
                self.latest_ask = market_trade.price;
                // 如果买方价格为0，初始化为相同的价格
                if self.latest_bid == 0.0 {
                    self.latest_bid = market_trade.price;
                }
            }
            _ => {
                // 处理无效的side值
                eprintln!("Invalid trade side: {}", market_trade.side);
            }
        }

        // 始终更新最新的交易价格
        self.latest_price = market_trade.price;
    }
}

impl From<&MarketTrade> for SingleLevelOrderBook
{
    fn from(market_trade: &MarketTrade) -> Self
    {
        let mut single_level_order_book = SingleLevelOrderBook { latest_bid: 0.0,
                                                                 latest_ask: 0.0,
                                                                 latest_price: market_trade.price };

        match Side::from_str(&market_trade.side) {
            | Ok(Side::Buy) => {
                single_level_order_book.latest_bid = market_trade.price; // 初始化买方价格
            }
            | Ok(Side::Sell) => {
                single_level_order_book.latest_ask = market_trade.price; // 初始化卖方价格
            }
            | _ => {
                // 处理无效的side值
                eprintln!("Invalid trade side: {}", market_trade.side);
            }
        }

        single_level_order_book
    }
}
