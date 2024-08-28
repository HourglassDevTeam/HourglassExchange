/// 在计算机科学和位运算中，“高位”和“低位”是指数字在二进制表示中的位置。
/// 
/// - **高位（Higher bits）**：指的是二进制数中靠近最左边的位置，即靠近数字开头的部分。这些位数的值较高，它们对最终的数值有更大的影响。
/// - **低位（Lower bits）**：指的是二进制数中靠近最右边的位置，即靠近数字末尾的部分。这些位数的值较低，对最终数值的影响较小。
/// 
/// ### 示例：32位的二进制数
/// 
/// ```
/// 11111111 00000000 11111111 00000000
/// |       |        |       |        |
/// 高位                             低位
/// ```
/// 
/// 在这个32位的二进制数字中，左边的 `11111111` 是最高的8位，它们对整个数值的贡献最大；而右边的 `00000000` 是最低的8位，它们对数值的贡献最小。
/// 
/// ### 在 `OrderId` 生成中的应用
/// 
/// 在当前的 `OrderId::new` 实现中：
/// 
///
/// let id = ((now & 0x1FFFFFFFFFF) << 23)
/// | ((machine_id & 0x3FF) << 13)
/// | ((counter & 0x3FF) << 3)
/// | (random_component & 0x7);

/// - `now` 被移位到最左边的高位。
/// - `machine_id` 位于中间。
/// - `counter` 被移动到靠右的位置。
/// - `random_component` 被放在最右边的最低位。
/// 
/// 这个设计使得 `now` 和 `machine_id` 对 `OrderId` 有最大的影响，因为它们在高位，而 `counter` 和 `random_component` 位于低位，因此它们的变化不会像高位那样显著影响 `OrderId` 的大小。这就是“高低”的含义——它指的是二进制数字的位置，以及这些位置对最终数字的影响。

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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间出现倒退")
            .as_millis() as u64;

        // 将随机组件降低到更低的位置
        let random_component: u64 = rand::thread_rng().gen_range(0..8192);

        // 调整 counter 的位移位置
        // 构建 OrderId: [timestamp:41 bits] [machine_id:10 bits] [counter:10 bits] [random:3 bits]
        let id = ((now & 0x1FFFFFFFFFF) << 23)
            | ((machine_id & 0x3FF) << 13)
            | ((counter & 0x3FF) << 3)
            | (random_component & 0x7);

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