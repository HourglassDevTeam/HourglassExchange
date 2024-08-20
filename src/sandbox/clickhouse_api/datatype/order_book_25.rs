use crate::sandbox::clickhouse_api::queries_operations::Row;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Row)]
pub struct OrderBook25
{
    pub exchange: String,
    pub symbol: String,
    pub timestamp: i64,
    pub local_timestamp: i64,
    pub asks_0_price: f64,
    pub asks_0_amount: f64,
    pub bids_0_price: f64,
    pub bids_0_amount: f64,
    pub asks_1_price: f64,
    pub asks_1_amount: f64,
    pub bids_1_price: f64,
    pub bids_1_amount: f64,
    pub asks_2_price: f64,
    pub asks_2_amount: f64,
    pub bids_2_price: f64,
    pub bids_2_amount: f64,
    pub asks_3_price: f64,
    pub asks_3_amount: f64,
    pub bids_3_price: f64,
    pub bids_3_amount: f64,
    pub asks_4_price: f64,
    pub asks_4_amount: f64,
    pub bids_4_price: f64,
    pub bids_4_amount: f64,
    pub asks_5_price: f64,
    pub asks_5_amount: f64,
    pub bids_5_price: f64,
    pub bids_5_amount: f64,
    pub asks_6_price: f64,
    pub asks_6_amount: f64,
    pub bids_6_price: f64,
    pub bids_6_amount: f64,
    pub asks_7_price: f64,
    pub asks_7_amount: f64,
    pub bids_7_price: f64,
    pub bids_7_amount: f64,
    pub asks_8_price: f64,
    pub asks_8_amount: f64,
    pub bids_8_price: f64,
    pub bids_8_amount: f64,
    pub asks_9_price: f64,
    pub asks_9_amount: f64,
    pub bids_9_price: f64,
    pub bids_9_amount: f64,
    pub asks_10_price: f64,
    pub asks_10_amount: f64,
    pub bids_10_price: f64,
    pub bids_10_amount: f64,
    pub asks_11_price: f64,
    pub asks_11_amount: f64,
    pub bids_11_price: f64,
    pub bids_11_amount: f64,
    pub asks_12_price: f64,
    pub asks_12_amount: f64,
    pub bids_12_price: f64,
    pub bids_12_amount: f64,
    pub asks_13_price: f64,
    pub asks_13_amount: f64,
    pub bids_13_price: f64,
    pub bids_13_amount: f64,
    pub asks_14_price: f64,
    pub asks_14_amount: f64,
    pub bids_14_price: f64,
    pub bids_14_amount: f64,
    pub asks_15_price: f64,
    pub asks_15_amount: f64,
    pub bids_15_price: f64,
    pub bids_15_amount: f64,
    pub asks_16_price: f64,
    pub asks_16_amount: f64,
    pub bids_16_price: f64,
    pub bids_16_amount: f64,
    pub asks_17_price: f64,
    pub asks_17_amount: f64,
    pub bids_17_price: f64,
    pub bids_17_amount: f64,
    pub asks_18_price: f64,
    pub asks_18_amount: f64,
    pub bids_18_price: f64,
    pub bids_18_amount: f64,
    pub asks_19_price: f64,
    pub asks_19_amount: f64,
    pub bids_19_price: f64,
    pub bids_19_amount: f64,
    pub asks_20_price: f64,
    pub asks_20_amount: f64,
    pub bids_20_price: f64,
    pub bids_20_amount: f64,
    pub asks_21_price: f64,
    pub asks_21_amount: f64,
    pub bids_21_price: f64,
    pub bids_21_amount: f64,
    pub asks_22_price: f64,
    pub asks_22_amount: f64,
    pub bids_22_price: f64,
    pub bids_22_amount: f64,
    pub asks_23_price: f64,
    pub asks_23_amount: f64,
    pub bids_23_price: f64,
    pub bids_23_amount: f64,
    pub asks_24_price: f64,
    pub asks_24_amount: f64,
    pub bids_24_price: f64,
    pub bids_24_amount: f64,
}
