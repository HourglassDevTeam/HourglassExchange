pub mod cid;

use std::fmt::Display;
use serde::{Deserialize, Serialize};

/// **OrderID**
///    - **定义和作用**：`OrderID` 通常由交易所生成，用于唯一标识某个订单。它是系统级的[标识符]，不同的[交易所]、不同的[订单类型]，都会生成不同的 `OrderID`。
///    - **设计合理性**：`OrderID` 的设计适合需要与交易所交互的场景，因为它通常是交易所内部或者[交易所]与[客户端]之间的标准标识符。
///  在扩展到To C端的Web应用和手机App时，`OrderID` 可以作为交易记录的[唯一标识符]，确保订单操作的[一致性]和[可追溯性]。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub String);

impl<Id> From<Id> for OrderId where Id: Display
{
    fn from(id: Id) -> Self
    {
        Self(id.to_string())
    }
}
