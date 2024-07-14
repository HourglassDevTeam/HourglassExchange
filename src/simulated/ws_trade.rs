// 回测专用的模拟WebSocket交易信息，只有4个字段
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsTrade {
    instId: String,
    side: String,
    px: String,
    ts: String,
}