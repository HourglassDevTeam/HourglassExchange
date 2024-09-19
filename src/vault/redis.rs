/// `PhantomData` 是 Rust 标准库中的一个零大小的类型，用于标记类型中的泛型参数，即使这些参数在运行时并未实际使用。
/// 这在 Rust 的类型系统中非常有用，尤其是在编译时确保类型安全。通常用于以下情况：
///
/// - **保证类型参数的存在**：在使用泛型时，即使泛型类型在运行时没有直接使用，`PhantomData` 也可以确保这个类型参数在编译时存在。
///   例如，在 `RedisVault` 和 `RedisVaultBuilder` 结构体中，`PhantomData<Statistic>` 确保了泛型 `Statistic` 的存在，即使它没有被显式地使用。
///
/// - **防止自动派生的实现**：Rust 编译器会自动为某些类型派生实现（如 `Send` 和 `Sync`），如果没有显式使用 `PhantomData`，可能会导致错误或不安全的实现。
///   `PhantomData` 可以帮助编译器正确处理这些类型的实现。
///
/// - **编译时类型安全性**：在编译时确保某些泛型参数的类型安全，即使这些参数在运行时没有直接的表现形式。
///
/// 在代码中的具体用途：
///
/// `PhantomData<Statistic>` 用于标记 `RedisVault` 和 `RedisVaultBuilder` 结构体中的 `Statistic` 泛型参数。
/// 这意味着即使 `Statistic` 在运行时没有被直接使用，Rust 的类型系统仍然会确保在编译时检查这个泛型参数的类型安全性。
use crate::error::ExchangeError;
use crate::{error::ExchangeError::RedisInitialisationError, hourglass::account::account_config::HourglassMode, vault::summariser::PositionSummariser};
use redis::Connection;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;
// use uuid::Uuid;
// use crate::common::account_positions::Position;
// use crate::common::account_positions::position_id::PositionId;

/// 用于通过 new() 构造函数方法构造 [`RedisVault`] 的配置。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
pub struct Config
{
    pub uri: String, // Redis连接的URI
}

/// 使用泛型类型 `Statistic` 的 Redis 仓库，`Statistic` 必须实现 `PositionSummariser`、`Serialize` 和 `DeserializeOwned`。
pub struct RedisVault<Statistic>
    where Statistic: PositionSummariser + Serialize + DeserializeOwned
{
    hourglass_mode: HourglassMode,                 // 仓库的配置，存储在仓库中
    _statistic_marker: PhantomData<Statistic>, // 用于类型标记的幻象数据
    #[allow(dead_code)]
    conn: Connection,
}

impl<Statistic> RedisVault<Statistic> where Statistic: PositionSummariser + Serialize + DeserializeOwned
{
    /// 使用提供的 Redis 连接和配置构造新的 [`RedisVault`] 组件。
    ///
    /// # 参数
    /// - `conn`: 与 Redis 数据库的连接。
    /// - `config`: 用于配置仓库的 `HourglassMode` 对象。
    ///
    /// # 返回
    /// 返回一个新的 `RedisVault` 实例，该实例可以用于与 Redis 数据库交互。
    pub fn new(conn: Connection, config: HourglassMode) -> Self
    {
        Self { hourglass_mode: config, // 存储提供的配置
               _statistic_marker: PhantomData,
               conn }
    }

    /// 构建器模式，用于构造仓库。
    ///
    /// # 返回
    /// 返回一个新的 `RedisVaultBuilder` 实例，该实例可以逐步配置并最终构建 `RedisVault`。
    pub fn builder() -> RedisVaultBuilder<Statistic>
    {
        RedisVaultBuilder::new()
    }

    /// 建立并返回一个 Redis 连接。
    ///
    /// # 参数
    /// - `cfg`: 包含 Redis 连接 URI 的 `Config` 对象。
    ///
    /// # 返回
    /// 返回一个 `Connection` 实例，用于与 Redis 服务器通信。
    pub fn setup_redis_connection(cfg: Config) -> Connection
    {
        redis::Client::open(cfg.uri).expect("无法创建 Redis 客户端").get_connection().expect("无法连接到 Redis")
    }

    /// 根据执行模式执行不同的操作。
    ///
    /// # 说明
    /// 该方法检查当前配置的执行模式，并基于此执行不同的操作。
    pub fn perform_action_based_on_mode(&self)
    {
        match self.hourglass_mode {
            | HourglassMode::Backtest => {
                todo!()
            }
            | HourglassMode::Online => {
                todo!()
            }
        }
    }
}

/// RedisVault 的构建器，使用泛型类型 `Statistic`，`Statistic` 必须实现 `PositionSummariser`、`Serialize` 和 `DeserializeOwned`。
#[derive(Default)]
pub struct RedisVaultBuilder<Statistic>
    where Statistic: PositionSummariser + Serialize + DeserializeOwned
{
    conn: Option<Connection>,                  // Redis 连接的可选值
    config: Option<HourglassMode>,               // 添加配置选项
    _statistic_marker: PhantomData<Statistic>, // 用于类型标记的幻象数据
}
impl<Statistic> RedisVaultBuilder<Statistic> where Statistic: PositionSummariser + Serialize + DeserializeOwned
{
    /// 构造新的 RedisVaultBuilder 实例。
    pub fn new() -> Self
    {
        Self { conn: None,
               config: None, // 初始化配置为 None
               _statistic_marker: PhantomData }
    }

    /// 设置 Redis 连接。
    pub fn conn(mut self, value: Connection) -> Self
    {
        self.conn = Some(value);
        self
    }

    /// 设置配置。
    pub fn config(mut self, value: HourglassMode) -> Self
    {
        self.config = Some(value);
        self
    }

    /// 构建 RedisVault 实例。
    pub fn build(self) -> Result<RedisVault<Statistic>, ExchangeError>
    {
        Ok(RedisVault { hourglass_mode: self.config.ok_or(RedisInitialisationError("config".to_string()))?, // 处理配置
                        _statistic_marker: PhantomData,
                        conn: self.conn.ok_or(RedisInitialisationError("connection".to_string()))? /* 处理连接 */ })
    }
}

// impl<Statistic> PositionProcessor for RedisVault<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned,
// {
//     fn add_open_position(&mut self, position: Position) -> Result<(), ExchangeError> {
//         // 将 Position 对象序列化为 JSON 字符串
//         let position_string = serde_json::to_string(&position)?;
//
//         // 将序列化后的 Position 存入 Redis
//         self.conn
//             .set(position.position_id.to_string(), position_string)
//             .map_err(|_| ExchangeError::WriteError)
//     }
//
//     fn get_open_position(
//         &mut self,
//         position_id: &PositionId,
//     ) -> Result<Option<Position>, ExchangeError> {
//         // 从 Redis 中获取与 position_id 关联的 Position 字符串
//         let position_value: String = self
//             .conn
//             .get(position_id.to_string())
//             .map_err(|_| ExchangeError::ReadError)?;
//
//         // 将 JSON 字符串反序列化为 Position 对象
//         Ok(Some(serde_json::from_str::<Position>(&position_value)?))
//     }
//
//     fn get_open_positions(
//         &mut self,
//         session_id: Uuid,
//         markets: impl Iterator<Item = Market>,
//     ) -> Result<Vec<Position>, ExchangeError> {
//         // 根据 markets 迭代器获取所有打开的 Position
//         markets
//             .filter_map(|market| {
//                 self.get_open_position(&determine_position_id(
//                     session_id,
//                     &market.exchange,
//                     &market.instrument,
//                 ))
//                     .transpose()
//             })
//             .collect()
//     }
//
//     fn remove_position(
//         &mut self,
//         position_id: &PositionId,
//     ) -> Result<Option<Position>, ExchangeError> {
//         // 获取并删除 Redis 中对应的 Position
//         let account_positions = self.get_open_position(position_id)?;
//
//         self.conn
//             .del(position_id.to_string())
//             .map_err(|_| ExchangeError::DeleteError)?;
//
//         Ok(account_positions)
//     }
//
//     fn set_exited_position(
//         &mut self,
//         session_id: Uuid,
//         account_positions: Position,
//     ) -> Result<(), ExchangeError> {
//         // 将已退出的 Position 推入 Redis 列表
//         self.conn
//             .lpush(
//                 determine_exited_positions_id(session_id),
//                 serde_json::to_string(&account_positions)?,
//             )
//             .map_err(|_| ExchangeError::WriteError)
//     }
//
//     fn get_exited_positions(&mut self, session_id: Uuid) -> Result<Vec<Position>, ExchangeError> {
//         // 获取 Redis 列表中的所有已退出 Position
//         let positions: Vec<String> = self.conn
//             .lrange(determine_exited_positions_id(session_id), 0, -1)
//             .map_err(|_| ExchangeError::ReadError)?;
//
//         positions
//             .iter()
//             .map(|position_str| serde_json::from_str::<Position>(position_str))
//             .collect::<Result<Vec<Position>, serde_json::Error>>()
//             .map_err(|_| ExchangeError::JsonSerDeError)
//     }
// }
