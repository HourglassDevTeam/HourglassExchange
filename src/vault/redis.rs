use crate::error::ExecutionError;
use crate::error::ExecutionError::RedisInitialisationError;
use crate::sandbox::account::account_config::{AccountConfig, SandboxMode};
use crate::vault::summariser::PositionSummariser;
use redis::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Configuration for constructing a [`RedisRepository`] via the new() constructor method.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub uri: String,
}


pub struct RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    config: AccountConfig, // Store the config within the repository
    _statistic_marker: PhantomData<Statistic>,
}

impl<Statistic> RedisRepository<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    /// Constructs a new [`RedisRepository`] component using the provided Redis connection and configuration.
    pub fn new(config: AccountConfig) -> Self {
        Self {
            config, // Store the provided config
            _statistic_marker: PhantomData,
        }
    }

    /// Builder pattern for constructing the repository.
    pub fn builder() -> RedisRepositoryBuilder<Statistic> {
        RedisRepositoryBuilder::new()
    }

    /// Establish & return a Redis connection.
    pub fn setup_redis_connection(cfg: Config) -> Connection {
        redis::Client::open(cfg.uri)
            .expect("Failed to create Redis client")
            .get_connection()
            .expect("Failed to connect to Redis")
    }

    /// A method that acts differently based on execution mode.
    pub fn perform_action_based_on_mode(&self) {
        match self.config.execution_mode {
            SandboxMode::Backtest => {
                // Perform action specific to backtesting
            }
            SandboxMode::RealTime => {
                // Perform action specific to real-time execution
            }
        }
    }
}
#[derive(Default)]
pub struct RedisRepositoryBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    conn: Option<Connection>,
    config: Option<AccountConfig>, // Add config option
    _statistic_marker: PhantomData<Statistic>,
}

impl<Statistic> RedisRepositoryBuilder<Statistic>
where
    Statistic: PositionSummariser + Serialize + DeserializeOwned,
{
    pub fn new() -> Self {
        Self {
            conn: None,
            config: None, // Initialize config as None
            _statistic_marker: PhantomData,
        }
    }

    pub fn conn(mut self, value: Connection) -> Self {
        self.conn = Some(value);
        self
    }

    pub fn config(mut self, value: AccountConfig) -> Self {
        self.config = Some(value);
        self
    }

    pub fn build(self) -> Result<RedisRepository<Statistic>, ExecutionError> {
        Ok(RedisRepository {
            config: self.config.ok_or(RedisInitialisationError("config".to_string()))?, // Handle config
            _statistic_marker: PhantomData,
        })
    }
}
