use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `Pending` 结构体表示订单在发送到客户端后尚未收到确认响应时的状态。
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Pending
{
    /// 标识订单是否为 `reduce_only`（仅减少头寸）。
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
    pub(crate) predicted_ts: i64,
}

/// 为 `Pending` 结构体实现 `PartialOrd` trait，允许对 `Pending` 结构体实例进行排序。
///
/// # 排序逻辑
///
/// 1. 首先按 `price` 进行比较。
/// 2. 如果 `price` 相等，则按 `size` 进行比较。
/// 3. 如果 `size` 也相等，则按 `predicted_ts` 进行比较。
/// 4. 如果 `predicted_ts` 也相等，则按 `reduce_only` 进行比较。
///
/// # 注意
///
/// 该实现目前主要用于测试目的，逻辑可能需要进一步优化。
impl PartialOrd for Pending
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        // 首先按价格比较
        match self.price.partial_cmp(&other.price) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // 如果价格相等，则按数量比较
        match self.size.partial_cmp(&other.size) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // 如果数量相等，则按预测时间戳比较
        match self.predicted_ts.partial_cmp(&other.predicted_ts) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // 最后按 `reduce_only` 字段比较
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}
