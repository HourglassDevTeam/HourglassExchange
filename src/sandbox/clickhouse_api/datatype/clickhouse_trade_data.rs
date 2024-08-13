use serde::{Deserialize, Serialize};
use crate::sandbox::clickhouse_api::queries_operations::Row;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ClickhouseTrade
{
    pub basequote: String,
    pub side: String,
    pub price: f64,
    pub timestamp: i64,
    pub amount: f64,
}