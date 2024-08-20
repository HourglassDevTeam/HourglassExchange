use crate::sandbox::clickhouse_api::queries_operations::Row;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ClickhousePublicTrade
{
    pub symbol: String, // NOTE : symbol = basequote
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
}
