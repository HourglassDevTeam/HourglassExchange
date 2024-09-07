use crate::{
    common::{trade::ClientTrade, Side},
    sandbox::clickhouse_api::queries_operations::Row,
};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct SingleLevelOrderBook
{
    latest_bid: f64,
    latest_ask: f64,
    latest_price: f64, // 记录最新的交易价格
}

#[allow(dead_code)]
impl SingleLevelOrderBook
{
    pub fn update_from_trade(&mut self, client_trade: ClientTrade)
    {
        match client_trade.side {
            | Side::Buy => {
                self.latest_bid = client_trade.price; // 更新买方价格
            }
            | Side::Sell => {
                self.latest_ask = client_trade.price; // 更新卖方价格
            }
        }
        self.latest_price = client_trade.price; // 更新最新的交易价格
    }
}
