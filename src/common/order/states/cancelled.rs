use crate::common::order::{identification::OrderId, states::open::Open, Order};
use serde::{Deserialize, Serialize};

/// 表示订单被取消后的状态。`Cancelled` 结构体通常用于标识一个订单已经从活动状态转变为取消状态。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Cancelled
{
    /// 被取消的订单ID。`OrderId` 用于唯一标识订单。
    pub id: OrderId,
}

/// 允许从其他类型转换为 `Cancelled` 结构体，前提是这些类型可以被转换为 `OrderId`。
impl<Id> From<Id> for Cancelled where Id: Into<OrderId>
{
    /// 将可以转换为 `OrderId` 的类型转换为 `Cancelled` 结构体。
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}

/// 允许从 `Order<Open>` 类型转换为 `Order<Cancelled>` 类型。
impl From<Order<Open>> for Order<Cancelled>
{
    /// 将 `Order<Open>` 转换为 `Order<Cancelled>`，保持订单的基本信息不变，只改变订单状态为取消。
    fn from(order: Order<Open>) -> Self
    {
        Self { kind: order.kind,
               exchange: order.exchange,
               instrument: order.instrument.clone(),
               cid: order.cid,
               client_ts: order.client_ts,
               side: order.side,
               state: Cancelled { id: order.state.id } }
    }
}
