use crate::common::order::OrderId;
use serde::{Deserialize, Serialize};

/// `FullyFill` 结构体表示订单完全成交的状态。
/// 完全成交状态意味着订单的所有数量已经被匹配和执行。
/// 在订单完全成交后，订单通常会从 `AccountOrders` 中删除。
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct FullyFill
{
    /// 完全成交的订单ID，唯一标识订单。
    pub id: OrderId,
    /// 完全成交时的价格。
    pub price: f64,
    /// 完全成交的订单数量。
    pub size: f64,
}

/// `PartialFill` 结构体表示订单部分成交的状态。
/// 部分成交状态意味着订单的部分数量已经被匹配和执行，
/// 但订单仍然在 `AccountOrders` 中保留，以待后续的成交。
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct PartialFill
{
    /// 部分成交的订单ID，唯一标识订单。
    pub id: OrderId,
    /// 部分成交时的价格。
    pub price: f64,
    /// 部分成交的订单数量。
    pub size: f64,
}
