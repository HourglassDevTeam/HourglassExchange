use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::common_skeleton::token::Token;
use crate::common_skeleton::{event::ClientOrderId, order::OrderKind};

/// 执行过程中可能遇到的错误。
#[derive(Error, PartialEq, Eq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExecutionError {
    /// 缺少属性，无法构建组件。
    #[error("[UniLinkExecution] : 由于缺少属性，无法构建组件: {0}")]
    BuilderIncomplete(String),

    /// 模拟交易所出错。
    #[error("[UniLinkExecution] : 模拟交易所错误: {0}")]
    Simulated(String),

    /// 余额不足，无法开单。
    #[error("[UniLinkExecution] : 符号{0}的余额不足，无法开单")]
    InsufficientBalance(Token),

    /// 找不到特定客户端订单ID的订单。
    #[error("[UniLinkExecution] : 未能找到具有客户端订单ID的订单: {0}")]
    OrderNotFound(ClientOrderId),

    /// 由于不支持的订单类型，无法开设订单。
    #[error("[UniLinkExecution] : 由于不支持的订单类型，无法开设订单: {0}")]
    UnsupportedOrderKind(OrderKind),

    /// 网络错误，无法连接到交易所。
    #[error("[UniLinkExecution] : 网络错误，无法连接到交易所: {0}")]
    NetworkError(String),

    /// 超时错误，操作超时。
    #[error("[UniLinkExecution] : 操作超时: {0}")]
    Timeout(String),

    /// 订单已存在。
    #[error("[UniLinkExecution] : 订单已存在: {0}")]
    OrderAlreadyExists(ClientOrderId),

    /// 订单被拒绝。
    #[error("[UniLinkExecution] : 订单被拒绝: {0}")]
    OrderRejected(String),

    /// 交易所维护中。
    #[error("[UniLinkExecution] : 交易所维护中，无法执行操作")]
    ExchangeMaintenance,

    /// 未知的交易所错误。
    #[error("[UniLinkExecution] : 未知的交易所错误: {0}")]
    UnknownExchangeError(String),

    /// 无效的交易对。
    #[error("[UniLinkExecution] : 无效的交易对: {0}")]
    InvalidTradingPair(String),

    /// API 限制，达到调用限制。
    #[error("[UniLinkExecution] : API 限制，达到调用限制")]
    ApiLimitReached,

    /// 权限不足。
    #[error("[UniLinkExecution] : 权限不足，无法执行操作")]
    InsufficientPermissions,

    /// 无效的签名。
    #[error("[UniLinkExecution] : 无效的签名")]
    InvalidSignature,

    /// 解析响应失败。
    #[error("[UniLinkExecution] : 解析响应失败: {0}")]
    ResponseParseError(String),

    /// 内部错误。
    #[error("[UniLinkExecution] : 内部错误: {0}")]
    InternalError(String),
}
