use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize, PartialOrd)]
pub struct PositionId(pub u64);

impl Display for PositionId
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

impl PositionId
{
    /// Generates a new `PositionId` using a variant of the snowflake algorithm.
    ///
    /// # Parameters
    ///
    /// - `timestamp`: The timestamp used for ID generation.
    /// - `machine_id`: The ID of the machine generating the ID, max value is 1023.
    /// - `counter`: The current counter value.
    ///
    /// # Returns
    ///
    /// Returns a unique `PositionId`.
    pub fn new(timestamp: u64, machine_id: u64, counter: u64) -> Self
    {
        let id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);
        PositionId(id)
    }

    /// Returns the internal `u64` value of the `PositionId`.
    pub fn value(&self) -> u64
    {
        self.0
    }
}

#[cfg(test)]
mod tests
{
    use super::PositionId;

    #[test]
    fn test_position_id_generation()
    {
        // 示例1: 测试一个常规的时间戳、机器ID 和 计数器组合
        let timestamp = 1625247123000; // 时间戳（单位：毫秒）
        let machine_id = 1; // 机器ID（假设这个机器的ID是1）
        let counter = 1; // 计数器（假设这是当前毫秒内的第一个请求）

        // 预期的ID值：根据算法，将时间戳左移22位，机器ID左移12位，然后与计数器合并
        let expected_id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);

        // 生成PositionId
        let position_id = PositionId::new(timestamp, machine_id, counter);

        // 断言生成的PositionId是否与预期值相等
        assert_eq!(position_id.value(), expected_id);

        // 示例2: 使用不同的时间戳、机器ID和计数器组合进行测试
        let timestamp = 1625247123001; // 更改时间戳为下一个毫秒
        let machine_id = 1023; // 使用最大值的机器ID（1023，二进制全1）
        let counter = 4095; // 使用最大值的计数器（4095，二进制全1）

        // 预期的ID值
        let expected_id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);

        // 生成PositionId并进行断言
        let position_id = PositionId::new(timestamp, machine_id, counter);
        assert_eq!(position_id.value(), expected_id);

        // 示例3: 测试边界情况，使用最大允许的时间戳、机器ID 和 计数器值
        let timestamp = 0x1FFFFFFFFFF; // 时间戳的最大值，约为 139 年
        let machine_id = 0x3FF; // 机器ID的最大值，二进制全1（1023）
        let counter = 0xFFF; // 计数器的最大值，二进制全1（4095）

        // 预期的ID值
        let expected_id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);

        // 生成PositionId并进行断言
        let position_id = PositionId::new(timestamp, machine_id, counter);
        assert_eq!(position_id.value(), expected_id);

        // 示例4: 测试最小值情况，所有输入参数都是0
        let timestamp = 0; // 时间戳最小值
        let machine_id = 0; // 机器ID最小值
        let counter = 0; // 计数器最小值

        // 预期的ID值
        let expected_id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);

        // 生成PositionId并进行断言
        let position_id = PositionId::new(timestamp, machine_id, counter);
        assert_eq!(position_id.value(), expected_id);
    }
}
