use crate::common::order::OrderId;
use serde::{Deserialize, Serialize};

/// `RequestCancel` 结构体表示一个取消订单的请求。
///
/// 在 `RequestCancel` 结构体中只记录了 `OrderId`，原因是取消订单操作通常只需要知道
/// 哪个订单需要被取消，无需其他额外信息。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel
{
    /// 要取消的订单的唯一标识符。
    pub id: OrderId,
}

/// 通过 `OrderId` 或其他可以转换为 `OrderId` 的类型生成 `RequestCancel` 实例。
///
/// 这个实现允许从不同的标识符类型（如字符串）直接创建 `RequestCancel` 实例。
impl<Id> From<Id> for RequestCancel where Id: Into<OrderId>
{
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}
