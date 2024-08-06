use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

pub mod balance; // 通用balance模块
pub mod datafeed;
pub mod event; // 定义通用事件和状态
pub mod friction;
pub mod instrument;
pub mod order;
pub mod status;
pub mod token;
pub mod trade;
pub(crate) mod position;

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
                | Side::Buy => "buy",
                | Side::Sell => "sell",
            }
        )
    }
}

impl Side {
    // 定义一个方法来切换交易方向
    pub fn toggle(&self) -> Self {
        match self {
            | Side::Buy => Side::Sell,
            | Side::Sell => Side::Buy,
        }
    }
}
