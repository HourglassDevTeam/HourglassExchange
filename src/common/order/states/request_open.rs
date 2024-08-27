use serde::{Deserialize, Serialize};
use std::cmp::Ordering;


/// 订单初始状态。发送到client进行操作
///
/// `RequestOpen` 用于表示一个初始订单状态。这个状态包含了订单的价格、大小，以及是否为 `reduce_only` 订单。
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RequestOpen
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
}

/// 通过实现 `PartialOrd`，我们可以基于 `RequestOpen` 的 `price`、`size` 和 `reduce_only` 字段进行排序和比较。
/// 这种实现假定价格是最重要的排序标准，其次是订单大小，最后是 `reduce_only` 标志。
///
/// FIXME: 这种比较逻辑可能不符合实际需求，应根据具体业务需求进行调整。
impl PartialOrd for RequestOpen
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        // 首先比较 `price`
        match self.price.partial_cmp(&other.price) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // 然后比较 `size`
        match self.size.partial_cmp(&other.size) {
            Some(Ordering::Equal) => {},
            non_eq => return non_eq,
        }
        // 最后比较 `reduce_only`
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}