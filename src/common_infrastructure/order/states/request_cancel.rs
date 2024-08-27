use crate::common_infrastructure::order::OrderId;
use serde::{Deserialize, Serialize};

/// 在RequestCancel结构体中只记录OrderId的原因主要是因为取消订单操作通常只需要知道哪个订单需要被取消。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct RequestCancel
{
    pub id: OrderId,
}

// 从Id直接生成RequestCancel
impl<Id> From<Id> for RequestCancel where Id: Into<OrderId>
{
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}
