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
