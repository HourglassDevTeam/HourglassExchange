use crate::common::order::{Order, OrderId, OrderRole};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 开放状态的订单
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Open
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
    pub filled_quantity: f64,
    pub order_role: OrderRole,
    pub received_ts: i64,
}

impl Open
{
    pub fn remaining_quantity(&self) -> f64
    {
        self.size - self.filled_quantity
    }
}

impl Ord for Order<Open>
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        match self.state.price.partial_cmp(&other.state.price) {
            | Some(ordering) => ordering,
            | None => panic!("[UniLinkExecution] : Failed to compare prices. One of the prices is NaN: self = {:?}, other = {:?}",
                             self.state.price, other.state.price),
        }
    }
}

impl PartialOrd for Order<Open>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

// 为Order<Open>实现Eq
impl Eq for Order<Open> {}
