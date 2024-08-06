use std::fs;

use crate::simulated_exchange::account::account_config::AccountConfig;

// 读取配置文件并初始化 AccountConfig
#[allow(dead_code)]
pub fn read_config_file(file_path: &str) -> Result<AccountConfig, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(file_path)?;
    let config: AccountConfig = toml::from_str(&config_content)?;
    Ok(config)
}