use serde::{Deserialize, Serialize};
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
    #[error("[UniLinkEx] : Unable to construct component due to missing property: {0}")]
    BuilderIncomplete(String),

    /// 模拟交易所出错。
    #[error("[UniLinkEx] : Sandbox error: {0}")]
    SandBox(String),

    /// 余额不足，无法开单。
    #[error("[UniLinkEx] : Insufficient balance for symbol {0}, unable to place order")]
    InsufficientBalance(Token),

    /// 找不到特定客户端订单ID的订单。
    #[error("[UniLinkEx] : Order with ClientOrderId not found: {0}")]
    RequestNotFound(RequestId),

    /// 找不到特定客户端订单ID的订单，并同时输出 `ClientOrderId` 和 `OrderId`（如果存在）。
    #[error("[UniLinkEx] : Order with ClientOrderId: {client_order_id:?}, and OrderId: {order_id:?} not found")]
    OrderNotFound
    {
        client_order_id: Option<ClientOrderId>, // 如果存在的话，输出 `ClientOrderId`
        order_id: Option<OrderId>,              // 如果存在的话，输出 `OrderId`
    },

    /// 由于不支持的订单类型，无法开设订单。
    #[error("[UniLinkEx] : Unsupported order type, unable to place order: {0}")]
    UnsupportedOrderKind(OrderInstruction),

    /// 网络错误，无法连接到交易所。
    #[error("[UniLinkEx] : Network error, unable to connect to exchange: {0}")]
    NetworkError(String),

    /// 网络错误，无法连接到交易所。
    #[error("[UniLinkEx] : ReponseSenderError, unable to connect to client.")]
    ReponseSenderError,

    /// 超时错误，操作超时。
    #[error("[UniLinkEx] : Operation timed out: {0}")]
    Timeout(String),

    /// 请求已存在。
    #[error("[UniLinkEx] : Order already exists: {0}")]
    RequestAlreadyExists(RequestId),

    /// 订单已存在。
    #[error("[UniLinkEx] : Order already exists: {0}")]
    OrderAlreadyExists(ClientOrderId),

    /// 订单被拒绝。
    #[error("[UniLinkEx] : Order rejected: {0}")]
    OrderRejected(String),

    /// 交易所维护中，无法执行操作。
    #[error("[UniLinkEx] : Exchange under maintenance, unable to perform operation")]
    ExchangeMaintenance,

    /// 无效的开单方向。
    #[error("[UniLinkEx] : Invalid order direction")]
    InvalidDirection,

    /// 无效的ID。
    #[error("[UniLinkEx] : Invalid ID")]
    InvalidID,

    /// 未知的交易所错误。
    #[error("[UniLinkEx] : Unknown exchange error: {0}")]
    UnknownExchangeError(String),

    /// 无效的交易对。
    #[error("[UniLinkEx] : Invalid trading pair: {0}")]
    InvalidTradingPair(String),

    /// 无效的日期。
    #[error("[UniLinkEx] : Invalid dates: {0}")]
    InvalidDates(String),

    /// API 限制，达到调用限制。
    #[error("[UniLinkEx] : API limit reached, unable to proceed")]
    ApiLimitReached,

    /// 权限不足，无法执行操作。
    #[error("[UniLinkEx] : Insufficient permissions to perform operation")]
    InsufficientPermissions,

    /// 无效的签名。
    #[error("[UniLinkEx] : Invalid signature provided")]
    InvalidSignature,

    /// 解析配置失败。
    #[error("[UniLinkEx] : Failed to parse configuration: {0}")]
    ConfigParseError(String),

    /// 配置缺少。
    #[error("[UniLinkEx] : Missing configuration")]
    ConfigMissing,

    /// 解析响应失败。
    #[error("[UniLinkEx] : Failed to parse response: {0}")]
    ResponseParseError(String),

    /// 内部错误。
    #[error("[UniLinkEx] : Internal error: {0}")]
    InternalError(String),

    /// 无效的金融工具。
    #[error("[UniLinkEx] : Invalid instrument: {0}")]
    InvalidInstrument(String),

    /// NotImplemented。
    #[error("[UniLinkEx] : Invalid instrument: {0}")]
    NotImplemented(String),

    #[error("[UniLinkEx] : Invalid RequestOpen: {0}")]
    InvalidRequestOpen(String),

    #[error("[UniLinkEx] : Invalid RequestCancel: {0}")]
    InvalidRequestCancel(String),

    #[error("[UniLinkEx] : Redis Initialisation Failure: {0}")]
    RedisInitialisationError(String),

    #[error("[UniLinkEx] : MarketEventChannelClosed")]
    MarketEventChannelClosed,

    #[error("[UniLinkEx] : InvalidLeverage")]
    InvalidLeverage(String),

    #[error("[UniLinkEx] : PostOnlyViolation")]
    PostOnlyViolation(String),

    #[error("[UniLinkEx] : ReduceOnlyViolation")]
    ReduceOnlyViolation,

    #[error("[UniLinkEx] : UnsupportedInstrumentKind")]
    UnsupportedInstrumentKind,

    #[error("[UniLinkEx] : Trying to update a non-existingPosition")]
    AttemptToUpdateNonExistingPosition,

    #[error("[UniLinkEx] : Trying to remove a non-existingPosition")]
    AttemptToRemoveNonExistingPosition,

    #[error("[UniLinkEx] : Redis fails to write.")]
    WriteError,

    #[error("[UniLinkEx] : Redis fails to read.")]
    ReadError,

    #[error("[UniLinkEx] : Redis fails to delete.")]
    DeleteError,

    #[error("[UniLinkEx] : Redis fails to Serialise/Deserialise.")]
    JsonSerDeError,

    #[error("[UniLinkEx] : Config Inheritance Not Allowed.")]
    ConfigInheritanceNotAllowed,
}
