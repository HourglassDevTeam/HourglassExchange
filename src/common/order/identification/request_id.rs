use fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize, PartialOrd)]
pub struct RequestId(pub u64);

impl Display for RequestId
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

impl RequestId
{
    /// 生成一个新的 `RequestId`，采用雪花算法的变种。
    ///
    /// # 参数
    ///
    /// - `timestamp`: 当前的时间戳，用于生成唯一的 ID。
    /// - `machine_id`: 用于标识生成 ID 的机器，最大值为 1023。
    /// - `counter`: 当前的请求计数器值。
    ///
    /// # 返回
    ///
    /// 返回一个唯一的 `RequestId`。
    pub fn new(timestamp: u64, machine_id: u64, counter: u64) -> Self
    {
        let id = ((timestamp & 0x1FFFFFFFFFF) << 22) | ((machine_id & 0x3FF) << 12) | (counter & 0xFFF);
        RequestId(id)
    }

    /// 返回 `RequestId` 的内部 `u64` 值。
    pub fn value(&self) -> u64
    {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    #[test]
    fn test_request_id_generation() {
        let machine_id = 1;
        let mut counter = 0;

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        let mut previous_id = RequestId::new(timestamp, machine_id, counter);

        for _ in 0..100 {
            counter += 1;
            let current_id = RequestId::new(timestamp, machine_id, counter);

            // 确保 ID 是递增的
            assert!(current_id > previous_id);

            // 更新 previous_id
            previous_id = current_id;
        }
    }

    #[test]
    fn test_request_id_uniqueness() {
        let machine_id = 1;
        let mut counter = 0;
        let mut ids = std::collections::HashSet::new();

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        for _ in 0..1000 {
            counter += 1;
            let id = RequestId::new(timestamp, machine_id, counter);
            assert!(ids.insert(id), "Duplicate RequestId generated: {}", id);
        }
    }
}
