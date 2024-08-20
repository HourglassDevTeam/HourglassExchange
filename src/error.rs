use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::common_infrastructure::{event::ClientOrderId, order::OrderKind, token::Token};

/// 执行过程中可能遇到的错误。
#[derive(Error, PartialEq, Eq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExecutionError
{
    /// 缺少属性，无法构建组件。
    #[error("[UniLinkExecution] : Unable to construct component due to missing property: {0}")]
    InitiatorIncomplete(String),

    /// 模拟交易所出错。
    #[error("[UniLinkExecution] : Sandbox error: {0}")]
    SandBox(String),

    /// 余额不足，无法开单。
    #[error("[UniLinkExecution] : Insufficient balance for symbol {0}, unable to place order")]
    InsufficientBalance(Token),

    /// 找不到特定客户端订单ID的订单。
    #[error("[UniLinkExecution] : Order with ClientOrderId not found: {0}")]
    OrderNotFound(ClientOrderId),

    /// 由于不支持的订单类型，无法开设订单。
    #[error("[UniLinkExecution] : Unsupported order type, unable to place order: {0}")]
    UnsupportedOrderKind(OrderKind),

    /// 网络错误，无法连接到交易所。
    #[error("[UniLinkExecution] : Network error, unable to connect to exchange: {0}")]
    NetworkError(String),

    /// 超时错误，操作超时。
    #[error("[UniLinkExecution] : Operation timed out: {0}")]
    Timeout(String),

    /// 订单已存在。
    #[error("[UniLinkExecution] : Order already exists: {0}")]
    OrderAlreadyExists(ClientOrderId),

    /// 订单被拒绝。
    #[error("[UniLinkExecution] : Order rejected: {0}")]
    OrderRejected(String),

    /// 交易所维护中，无法执行操作。
    #[error("[UniLinkExecution] : Exchange under maintenance, unable to perform operation")]
    ExchangeMaintenance,

    /// 无效的开单方向。
    #[error("[UniLinkExecution] : Invalid order direction")]
    InvalidDirection,

    /// 未知的交易所错误。
    #[error("[UniLinkExecution] : Unknown exchange error: {0}")]
    UnknownExchangeError(String),

    /// 无效的交易对。
    #[error("[UniLinkExecution] : Invalid trading pair: {0}")]
    InvalidTradingPair(String),

    /// 无效的日期。
    #[error("[UniLinkExecution] : Invalid dates: {0}")]
    InvalidDates(String),

    /// API 限制，达到调用限制。
    #[error("[UniLinkExecution] : API limit reached, unable to proceed")]
    ApiLimitReached,

    /// 权限不足，无法执行操作。
    #[error("[UniLinkExecution] : Insufficient permissions to perform operation")]
    InsufficientPermissions,

    /// 无效的签名。
    #[error("[UniLinkExecution] : Invalid signature provided")]
    InvalidSignature,

    /// 解析配置失败。
    #[error("[UniLinkExecution] : Failed to parse configuration: {0}")]
    ConfigParseError(String),

    /// 配置缺少。
    #[error("[UniLinkExecution] : Missing configuration: {0}")]
    ConfigMissing(String),

    /// 解析响应失败。
    #[error("[UniLinkExecution] : Failed to parse response: {0}")]
    ResponseParseError(String),

    /// 内部错误。
    #[error("[UniLinkExecution] : Internal error: {0}")]
    InternalError(String),

    /// 无效的金融工具。
    #[error("[UniLinkExecution] : Invalid instrument: {0}")]
    InvalidInstrument(String),
}
