// 回测专用的模拟WebSocket交易信息，只有4个字段
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsTrade
{
    #[serde(alias = "instrument_id", alias = "inst_id")]
    instId: String,
    #[serde(alias = "side")]
    side: String,
    #[serde(alias = "price")]
    px: String,
    #[serde(alias = "timestamp")]
    ts: String,
}
