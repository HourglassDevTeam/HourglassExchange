use crate::error::ExecutionError;
use crate::error::ExecutionError::RedisInitialisationError;
use crate::sandbox::account::account_config::{AccountConfig, SandboxMode};
use crate::vault::summariser::PositionSummariser;
use redis::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// 用于通过 new() 构造函数方法构造 [`RedisVault`] 的配置。
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub uri: String, // Redis连接的URI
}

/// 使用泛型类型 `Statistic` 的 Redis 仓库，`Statistic` 必须实现 `PositionSummariser`、`Serialize` 和 `DeserializeOwned`。
pub struct RedisVault<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    config: AccountConfig, // 仓库的配置，存储在仓库中
    _statistic_marker: PhantomData<Statistic>, // 用于类型标记的幻象数据
    #[allow(dead_code)]
    conn: Connection
}

impl<Statistic> RedisVault<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    /// 使用提供的 Redis 连接和配置构造新的 [`RedisVault`] 组件。
    pub fn new(conn: Connection,config: AccountConfig) -> Self {
        Self {
            config, // 存储提供的配置
            _statistic_marker: PhantomData,
            conn,

        }
    }

    /// 构建器模式，用于构造仓库。
    pub fn builder() -> RedisVaultBuilder<Statistic> {
        RedisVaultBuilder::new()
    }

    /// 建立并返回一个 Redis 连接。
    pub fn setup_redis_connection(cfg: Config) -> Connection {
        redis::Client::open(cfg.uri)
            .expect("无法创建 Redis 客户端")
            .get_connection()
            .expect("无法连接到 Redis")
    }

    /// 根据执行模式执行不同的操作。
    pub fn perform_action_based_on_mode(&self) {
        match self.config.execution_mode {
            SandboxMode::Backtest => {
                todo!()
            }
            SandboxMode::RealTime => {
                todo!()
            }
        }
    }
}

/// RedisVault 的构建器，使用泛型类型 `Statistic`，`Statistic` 必须实现 `PositionSummariser`、`Serialize` 和 `DeserializeOwned`。
#[derive(Default)]
pub struct RedisVaultBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    conn: Option<Connection>, // Redis 连接的可选值
    config: Option<AccountConfig>, // 添加配置选项
    _statistic_marker: PhantomData<Statistic>, // 用于类型标记的幻象数据
}
impl<Statistic> RedisVaultBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    /// 构造新的 RedisVaultBuilder 实例。
    pub fn new() -> Self {
        Self {
            conn: None,
            config: None, // 初始化配置为 None
            _statistic_marker: PhantomData,
        }
    }

    /// 设置 Redis 连接。
    pub fn conn(mut self, value: Connection) -> Self {
        self.conn = Some(value);
        self
    }

    /// 设置配置。
    pub fn config(mut self, value: AccountConfig) -> Self {
        self.config = Some(value);
        self
    }

    /// 构建 RedisVault 实例。
    pub fn build(self) -> Result<RedisVault<Statistic>, ExecutionError> {
        Ok(RedisVault {
            config: self.config.ok_or(RedisInitialisationError("config".to_string()))?, // 处理配置
            _statistic_marker: PhantomData,
            conn: self.conn.ok_or(RedisInitialisationError("connection".to_string()))?, // 处理连接
        })
    }
}


//
// impl<Statistic> PositionHandler for RedisVault<Statistic>
// where
//     Statistic: PositionSummariser + Serialize + DeserializeOwned,
// {
//     fn set_open_position(&mut self, position: Position) -> Result<(), ExecutionError> {
//         // 将 Position 对象序列化为 JSON 字符串
//         let position_string = serde_json::to_string(&position)?;
//
//         // 将序列化后的 Position 存入 Redis
//         self.conn
//             .set(position.position_id.to_string(), position_string)
//             .map_err(|_| ExecutionError::WriteError)
//     }
//
//     fn get_open_position(
//         &mut self,
//         position_id: &PositionId,
//     ) -> Result<Option<Position>, ExecutionError> {
//         // 从 Redis 中获取与 position_id 关联的 Position 字符串
//         let position_value: String = self
//             .conn
//             .get(position_id.to_string())
//             .map_err(|_| ExecutionError::ReadError)?;
//
//         // 将 JSON 字符串反序列化为 Position 对象
//         Ok(Some(serde_json::from_str::<Position>(&position_value)?))
//     }
//
//     fn get_open_positions(
//         &mut self,
//         instance_id: Uuid,
//         markets: impl Iterator<Item = Market>,
//     ) -> Result<Vec<Position>, ExecutionError> {
//         // 根据 markets 迭代器获取所有打开的 Position
//         markets
//             .filter_map(|market| {
//                 self.get_open_position(&determine_position_id(
//                     instance_id,
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
//     ) -> Result<Option<Position>, ExecutionError> {
//         // 获取并删除 Redis 中对应的 Position
//         let position = self.get_open_position(position_id)?;
//
//         self.conn
//             .del(position_id.to_string())
//             .map_err(|_| ExecutionError::DeleteError)?;
//
//         Ok(position)
//     }
//
//     fn set_exited_position(
//         &mut self,
//         instance_id: Uuid,
//         position: Position,
//     ) -> Result<(), ExecutionError> {
//         // 将已退出的 Position 推入 Redis 列表
//         self.conn
//             .lpush(
//                 determine_exited_positions_id(instance_id),
//                 serde_json::to_string(&position)?,
//             )
//             .map_err(|_| ExecutionError::WriteError)
//     }
//
//     fn get_exited_positions(&mut self, instance_id: Uuid) -> Result<Vec<Position>, ExecutionError> {
//         // 获取 Redis 列表中的所有已退出 Position
//         let positions: Vec<String> = self.conn
//             .lrange(determine_exited_positions_id(instance_id), 0, -1)
//             .map_err(|_| ExecutionError::ReadError)?;
//
//         positions
//             .iter()
//             .map(|position_str| serde_json::from_str::<Position>(position_str))
//             .collect::<Result<Vec<Position>, serde_json::Error>>()
//             .map_err(ExecutionError::JsonSerDeError)
//     }
// }