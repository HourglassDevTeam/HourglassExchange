use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::{Deserialize, Serialize};
use crate::common::instrument::Instrument;

/// PositionId 结构体，存储为 `u64`
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize,Hash,Eq)]
pub struct PositionId(pub u64);

impl PositionId {
    /// 生成 `PositionId`，使用 `Instrument` 和 `timestamp`
    pub fn new(instrument: &Instrument, timestamp: i64) -> Self {
        let instrument_hash = instrument.hash_as_u64();
        let timestamp_u64 = timestamp as u64;

        // 将 `instrument_hash` 放在高位，`timestamp_u64` 放在低位
        let position_id = (instrument_hash << 32) | (timestamp_u64 & 0xFFFF_FFFF);
        Self(position_id)
    }

    /// 获取 `u64` 值
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Instrument {
    /// 将 `Instrument` 转换为一个 `u64`，通过哈希
    pub fn hash_as_u64(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.base.hash(&mut hasher);
        self.quote.hash(&mut hasher);
        self.kind.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::common::instrument::kind::InstrumentKind;
    use super::*;

    #[test]
    fn test_position_id() {
        let instrument = Instrument::new("BTC", "USDT", InstrumentKind::Perpetual);
        let timestamp = 1625247600;

        let position_id = PositionId::new(&instrument, timestamp);

        // 直接打印 `PositionId`
        println!("PositionId: {}", position_id.as_u64());

        // 确保 `PositionId` 生成的是非零的 `u64`
        assert!(position_id.as_u64() > 0);
    }
}
