use crate::common_infrastructure::order::OrderId;
use serde::{Deserialize, Serialize};

/// 完全成交状态的订单, FullFill 以后在account层面 [AccountOrders] 删除对应open订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FullyFill
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
}

/// 完全成交状态的订单,FullFill 以后在account层面 [AccountOrders] 仍然保留open订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PartialFill
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
}
