use
serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::common::{
    order::{
        identification::{client_order_id::ClientOrderId, request_id::RequestId, OrderId},
        order_instructions::OrderInstruction,
    },
    token::Token,
};

/// 执行过程中可能遇到的错误。
#[derive(Error, PartialEq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum ExchangeError
{
    /// 缺少属性，无法构建组件。
    #[error("Unable to construct component due to missing property: {0}")]
    BuilderIncomplete(String),

    /// 模拟交易所出错。
    #[error("Hourglass error: {0}")]
    Hourglass(String),

    /// 余额不足，无法开单。
    #[error("Insufficient balance for symbol {0}, unable to place order")]
    InsufficientBalance(Token),

    /// 找不到特定客户端订单ID的订单。
    #[error("Order with ClientOrderId not found: {0}")]
    RequestNotFound(RequestId),

    /// 找不到特定客户端订单ID的订单，并同时输出 `ClientOrderId` 和 `OrderId`（如果存在）。
    #[error("Order with ClientOrderId: {client_order_id:?}, and OrderId: {order_id:?} not found")]
    OrderNotFound
    {
        client_order_id: Option<ClientOrderId>, // 如果存在的话，输出 `ClientOrderId`
        order_id: Option<OrderId>,              // 如果存在的话，输出 `OrderId`
    },

    /// 由于不支持的订单类型，无法开设订单。
    #[error("Unsupported order type, unable to place order: {0}")]
    UnsupportedOrderKind(OrderInstruction),

    /// 网络错误，无法连接到交易所。
    #[error("Network error, unable to connect to exchange: {0}")]
    NetworkError(String),

    /// 网络错误，无法连接到交易所。
    #[error("ReponseSenderError, unable to connect to client.")]
    ReponseSenderError,

    /// 超时错误，操作超时。
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// 请求已存在。
    #[error("Order already exists: {0}")]
    RequestAlreadyExists(RequestId),

    /// 订单已存在。
    #[error("Order already exists: {0}")]
    OrderAlreadyExists(ClientOrderId),

    /// 订单被拒绝。
    #[error("Order rejected: {0}")]
    OrderRejected(String),

    /// 交易所维护中，无法执行操作。
    #[error("Exchange under maintenance, unable to perform operation")]
    ExchangeMaintenance,

    /// 无效的开单方向。
    #[error("Invalid order direction")]
    InvalidDirection,

    /// 无效的ID。
    #[error("Invalid ID")]
    InvalidID,

    /// 未知的交易所错误。
    #[error("Unknown exchange error: {0}")]
    UnknownExchangeError(String),

    /// 无效的交易对。
    #[error("Invalid trading pair: {0}")]
    InvalidTradingPair(String),

    /// 无效的日期。
    #[error("Invalid dates: {0}")]
    InvalidDates(String),

    /// API 限制，达到调用限制。
    #[error("API limit reached, unable to proceed")]
    ApiLimitReached,

    /// 权限不足，无法执行操作。
    #[error("Insufficient permissions to perform operation")]
    InsufficientPermissions,

    /// 无效的签名。
    #[error("Invalid signature provided")]
    InvalidSignature,

    /// 解析配置失败。
    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),

    /// 配置缺少。
    #[error("Missing configuration")]
    ConfigMissing,

    /// 解析响应失败。
    #[error("Failed to parse response: {0}")]
    ResponseParseError(String),

    /// 内部错误。
    #[error("Internal error: {0}")]
    InternalError(String),

    /// 无效的金融工具。
    #[error("Invalid instrument: {0}")]
    InvalidInstrument(String),

    /// NotImplemented。
    #[error("Invalid instrument: {0}")]
    NotImplemented(String),

    #[error("Invalid RequestOpen: {0}")]
    InvalidRequestOpen(String),

    #[error("Invalid RequestCancel: {0}")]
    InvalidRequestCancel(String),

    #[error("Redis Initialisation Failure: {0}")]
    RedisInitialisationError(String),

    #[error("MarketEventChannelClosed")]
    MarketEventChannelClosed,

    #[error("InvalidLeverage")]
    InvalidLeverage(String),

    #[error("PostOnlyViolation")]
    PostOnlyViolation(String),

    #[error("ReduceOnlyViolation")]
    ReduceOnlyViolation,

    #[error("UnsupportedInstrumentKind")]
    UnsupportedInstrumentKind,

    #[error("Trying to update a non-existingPosition")]
    AttemptToUpdateNonExistingPosition,

    #[error("Trying to remove a non-existingPosition")]
    AttemptToRemoveNonExistingPosition,

    #[error("Redis fails to write.")]
    WriteError,

    #[error("Redis fails to read.")]
    ReadError,

    #[error("Redis fails to delete.")]
    DeleteError,

    #[error("Redis fails to Serialise/Deserialise.")]
    JsonSerDeError,

    #[error("Config Inheritance Not Allowed.")]
    ConfigInheritanceNotAllowed,

    #[error("AuthenticationFailed.")]
    AuthenticationFailed,

    #[error("InvalidTradeSize.")]
    InvalidTradeSize,

    #[error("InvalidCredentials.")]
    InvalidCredentials,

    #[error("InvalidSession.")]
    InvalidSession,

    #[error("DatabaseError.")]
    DatabaseError,

    #[error("PasswordHashError.")]
    PasswordHashError,
}
