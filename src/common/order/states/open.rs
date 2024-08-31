use crate::common::order::{identification::OrderId, Order, OrderRole};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `Open` 结构体表示订单在开放状态下的详细信息。
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Open
{
    pub id: OrderId,
    pub price: f64,
    pub size: f64,
    pub filled_quantity: f64,
    pub order_role: OrderRole,
}

impl Open
{
    /// 计算订单的剩余数量。
    ///
    /// # 返回值
    /// 返回订单的剩余数量（`size - filled_quantity`）。
    pub fn remaining_quantity(&self) -> f64
    {
        self.size - self.filled_quantity
    }
}

impl Ord for Order<Open>
{
    /// 实现 `Ord` trait 以便按订单的价格进行排序。
    ///
    /// # 注意
    /// 如果价格包含 `NaN`，将会引发 panic。
    ///
    /// # 参数
    /// - `other`: 要比较的另一个 `Order<Open>` 实例。
    ///
    /// # 返回值
    /// 返回 `Ordering`，指示当前实例相对于 `other` 是小于、等于还是大于。
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
    /// 实现 `PartialOrd` trait，使用 `cmp` 方法进行比较。
    ///
    /// # 参数
    /// - `other`: 要比较的另一个 `Order<Open>` 实例。
    ///
    /// # 返回值
    /// 返回 `Option<Ordering>`，指示当前实例相对于 `other` 是小于、等于还是大于。
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

/// 为 `Order<Open>` 实现 `Eq` trait，以支持完全相等的比较。
impl Eq for Order<Open> {}
