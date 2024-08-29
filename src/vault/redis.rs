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
}

impl<Statistic> RedisVault<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    /// 使用提供的 Redis 连接和配置构造新的 [`RedisVault`] 组件。
    pub fn new(config: AccountConfig) -> Self {
        Self {
            config, // 存储提供的配置
            _statistic_marker: PhantomData,
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
                // 进行特定于回测的操作
            }
            SandboxMode::RealTime => {
                // 进行特定于实时执行的操作
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
        })
    }
}
