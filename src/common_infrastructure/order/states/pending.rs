use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// 发送RequestOpen到client后尚未收到确认响应时的状态
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Pending
{
    pub reduce_only: bool,
    pub price: f64,
    pub size: f64,
    pub(crate) predicted_ts: i64,
}

/// FIXME this comparison is now only for the tests below but could be ill-logic.
impl PartialOrd for Pending
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
        // Then compare by `predicted_ts`
        match self.predicted_ts.partial_cmp(&other.predicted_ts) {
            | Some(Ordering::Equal) => {}
            | non_eq => return non_eq,
        }
        // Finally compare by `reduce_only`
        Some(self.reduce_only.cmp(&other.reduce_only))
    }
}
