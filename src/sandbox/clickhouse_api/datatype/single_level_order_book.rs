use crate::{
    common::Side,
    sandbox::clickhouse_api::{datatype::clickhouse_trade_data::MarketTrade, queries_operations::Row},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct SingleLevelOrderBook
{
    pub(crate) latest_bid: f64,
    pub(crate) latest_ask: f64,
    pub(crate) latest_price: f64, // 记录最新的交易价格
}

pub trait OrderBookUpdater
{
    fn update_from_trade(&mut self, market_trade: &MarketTrade);
}

impl OrderBookUpdater for SingleLevelOrderBook
{
    fn update_from_trade(&mut self, market_trade: &MarketTrade)
    {
        match Side::from_str(&market_trade.side) {
            | Ok(Side::Buy) => {
                self.latest_bid = market_trade.price; // 更新买方价格
            }
            | Ok(Side::Sell) => {
                self.latest_ask = market_trade.price; // 更新卖方价格
            }
            | _ => {
                // 处理无效的side值
                eprintln!("Invalid trade side: {}", market_trade.side);
            }
        }
        self.latest_price = market_trade.price; // 更新最新的交易价格
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
