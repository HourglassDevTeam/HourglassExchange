pub mod client_order_id;
pub mod machine_id;
pub mod request_order_id;

use std::fmt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

/// **OrderID**
///    - **定义和作用**：`OrderID` 通常由交易所生成，用于唯一标识某个订单。它是系统级的[标识符]，不同的[交易所]、不同的[订单类型]，都会生成不同的 `OrderID`。
///    - **设计合理性**：`OrderID` 的设计适合需要与交易所交互的场景，因为它通常是交易所内部或者[交易所]与[客户端]之间的标准标识符。
///  在扩展到To C端的Web应用和手机App时，`OrderID` 可以作为交易记录的[唯一标识符]，确保订单操作的[一致性]和[可追溯性]。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OrderId(pub u64);

impl Display for OrderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


impl OrderId {
    /// 生成一个具有更高安全性要求的 `OrderId`。
    ///
    /// # 参数
    /// - `machine_id`: 用于生成ID的机器唯一标识符，最大值为1023。
    /// - `counter`: 当前的计数器值，用于确保ID的唯一性。
    ///
    /// # 返回值
    /// - 返回一个唯一且安全的 `OrderId`。
    pub fn new(machine_id: u64, counter: u64) -> Self {
        // 获取当前时间的毫秒数，并确保时间是从 UNIX_EPOCH 之后的。
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间出现倒退")
            .as_millis() as u64;

        // 生成一个随机组件，用于增加ID的唯一性和不可预测性。
        let random_component: u64 = rand::thread_rng().gen_range(0..8192);

        // 构建 OrderId: [timestamp:41 bits] [machine_id:10 bits] [random:3 bits] [counter:10 bits]
        let id = ((now & 0x1FFFFFFFFFF) << 23)
            | ((machine_id & 0x3FF) << 13)
            | ((random_component & 0x7) << 10)
            | (counter & 0x3FF);

        OrderId(id)
    }

    /// 返回 `OrderId` 的内部 `u64` 值。
    ///
    /// # 返回值
    /// - 返回内部存储的 `u64` 类型的ID值。
    pub fn value(&self) -> u64 {
        self.0
    }
}