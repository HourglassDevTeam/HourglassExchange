use crate::{error::ExecutionError, simulated_exchange::account::account_config::AccountConfig};
use std::fs;

// 读取配置文件
pub fn read_config_file(file_path: &str) -> Result<AccountConfig, ExecutionError>
{
    let config_content = fs::read_to_string(file_path).map_err(ExecutionError::from)?;
    let config: AccountConfig = toml::from_str(&config_content).map_err(ExecutionError::from)?;
    Ok(config)
}

impl From<std::io::Error> for ExecutionError
{
    fn from(err: std::io::Error) -> Self
    {
        ExecutionError::InternalError(format!("IO error: {}", err))
    }
}

impl From<toml::de::Error> for ExecutionError
{
    fn from(err: toml::de::Error) -> Self
    {
        ExecutionError::ResponseConfigError(format!("TOML error: {}", err))
    }
}
