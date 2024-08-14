use serde::{Deserialize, Serialize};
use crate::sandbox::clickhouse_api::queries_operations::Row;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ClickhousePublicTrade
{
    pub symbol: String, //NOTE : symblo = basequote
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
}