use std::str::FromStr;
use crate::{
    common:: Side,
    sandbox::clickhouse_api::queries_operations::Row,
};
use serde::{Deserialize, Serialize};
use crate::sandbox::clickhouse_api::datatype::clickhouse_trade_data::MarketTrade;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct SingleLevelOrderBook
{
    latest_bid: f64,
    latest_ask: f64,
    latest_price: f64, // 记录最新的交易价格
}

#[allow(dead_code)]
impl SingleLevelOrderBook {
    pub fn update_from_trade(&mut self, client_trade: &MarketTrade) {
        match Side::from_str(&client_trade.side) {
            Ok(Side::Buy) => {
                self.latest_bid = client_trade.price; // 更新买方价格
            }
            Ok(Side::Sell) => {
                self.latest_ask = client_trade.price; // 更新卖方价格
            }
            _ => {
                // 处理无效的side值
                eprintln!("Invalid trade side: {}", client_trade.side);
            }
        }
        self.latest_price = client_trade.price; // 更新最新的交易价格
    }
}
