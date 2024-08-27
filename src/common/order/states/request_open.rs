use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 订单初始状态。发送到client进行操作
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct RequestOpen
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
}

/// FIXME this comparison is now only for the tests below but could be ill-logic.
impl PartialOrd for RequestOpen
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        // First compare by `price`
        match self.price.partial_cmp(&other.price) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // Then compare by `size`
        match self.size.partial_cmp(&other.size) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // Finally compare by `reduce_only`
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}
