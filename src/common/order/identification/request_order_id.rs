use std::sync::{LazyLock, atomic::{AtomicU64, Ordering}};
use std::time::{SystemTime, UNIX_EPOCH};


#[allow(dead_code)]
static LOCAL_COUNTER: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestId(u64);

#[allow(dead_code)]
impl RequestId {
    /// 生成一个新的 RequestId，采用雪花算法（Snowflake ID）的变种。
    ///
    /// # 参数
    ///
    /// - `machine_id`: 用于标识生成 ID 的机器，最大值为 1023。
    ///
    /// # 返回
    ///
    /// 返回一个唯一的 `RequestId`。
    ///
    /// # 说明
    ///
    /// 该 ID 的组成部分为：
    /// - 41 位：当前的时间戳，表示自定义纪元（epoch）以来的毫秒数。
    /// - 10 位：机器 ID，用于区分不同的节点或机器。
    /// - 12 位：自增序列，确保在同一毫秒内生成的 ID 唯一。
    ///
    /// # 注意
    ///
    /// `machine_id` 可以从配置文件中读取，也可以通过哈希服务器的 MAC 地址或 IP 地址生成，确保在分布式系统中唯一。
    pub fn new(machine_id: u16) -> Self {
        // 获取当前的时间戳（毫秒），并减去自定义的纪元（通常是一个固定的时间点）
        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;

        // 获取全局计数器的当前值并递增
        let counter = LOCAL_COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFF;

        // 生成 RequestId: [timestamp:41 bits] [machine_id:10 bits] [counter:12 bits]
        let id = ((now & 0x1FFFFFFFFFF) << 22) | ((machine_id as u64 & 0x3FF) << 12) | counter;

        RequestId(id)
    }

    /// 返回 RequestId 的内部 u64 值。
    pub fn value(&self) -> u64 {
        self.0
    }
}
