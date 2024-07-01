use cerebro_integration::model::instrument::symbol::Symbol;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{order::OrderKind, ClientOrderId};

/// 执行过程中可能遇到的错误。
#[derive(Error, PartialEq, Eq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExecutionError {
    /// 缺少属性，无法构建组件。
    #[error("[TideBroker] : 由于缺少属性，无法构建组件: {0}")]
    BuilderIncomplete(String),

    /// 模拟交易所出错。
    #[error("[TideBroker] : 模拟交易所错误: {0}")]
    Simulated(String),

    /// 余额不足，无法开单。
    #[error("[TideBroker] : 符号{0}的余额不足，无法开单")]
    InsufficientBalance(Symbol),

    /// 找不到特定客户端订单ID的订单。
    #[error("[TideBroker] : 未能找到具有客户端订单ID的订单: {0}")]
    OrderNotFound(ClientOrderId),

    /// 由于不支持的订单类型，无法开设订单。
    #[error("[TideBroker] : 由于不支持的订单类型，无法开设订单: {0}")]
    UnsupportedOrderKind(OrderKind),
}
