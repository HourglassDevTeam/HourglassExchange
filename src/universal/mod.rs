use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

pub mod balance; // 通用balance模块
pub mod data;
pub mod event; // 定义通用事件和状态
pub mod friction;
pub mod instrument;
pub mod order; // 通用订单模块
pub mod position;
pub mod status;
pub mod token;
pub mod trade;

// 通用交易模块
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Side {
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
        )
    }
}
