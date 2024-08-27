use crate::common_infrastructure::order::states::open::Open;
use crate::common_infrastructure::order::{Order, OrderId};
use serde::{Deserialize, Serialize};

/// 构建订单在被取消后的状态
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Cancelled
{
    pub id: OrderId,
}

impl<Id> From<Id> for Cancelled
where
    Id: Into<OrderId>,
{
    fn from(id: Id) -> Self
    {
        Self { id: id.into() }
    }
}

impl From<Order<Open>> for Order<Cancelled>
{
    fn from(order: Order<Open>) -> Self
    {
        Self {
            kind: order.kind,
            exchange: order.exchange,
            instrument: order.instrument.clone(),
            client_order_id: order.client_order_id,
            client_ts: order.client_ts,
            side: order.side,
            state: Cancelled { id: order.state.id },
        }
    }
}