use chrono::Duration;

pub mod dispersion;
pub mod error;
pub mod metrics;
pub mod summary;
pub mod welford_online;
use serde::{Deserialize, Deserializer, Serializer};

/// 将 [`Duration`] 序列化为表示秒数的 `u64`。
///
/// # 原理介绍
/// 在序列化过程中，我们需要将 Rust 的 `Duration` 类型转换为可以表示的简单数字格式，
/// 例如 JSON 中的整数。这个函数通过将 `Duration` 转换为秒数，并将其序列化为 `u64` 类型，
pub fn se_duration_as_secs<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(duration.num_seconds())
}

/// 将表示秒数的数字反序列化为 [`Duration`]。
///
/// # 原理介绍
/// 在反序列化过程中，我们需要将简单的数字格式（例如 JSON 中的整数）转换回 Rust 的 `Duration` 类型。
/// 这个函数接收一个表示秒数的整数，并将其转换为 `Duration` 类型。
pub fn de_duration_from_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: i64 = Deserialize::deserialize(deserializer)?;
    Ok(Duration::seconds(seconds))
}
