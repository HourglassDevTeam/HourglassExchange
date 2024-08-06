use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::common_skeleton::instrument::Instrument;
use crate::common_skeleton::instrument::kind::InstrumentKind;
use crate::common_skeleton::position::{PositionDirectionMode, PositionMarginMode};
use crate::simulated_exchange::account::account_config::{AccountConfig, CommissionLevel, CommissionRates, MarginMode};

// 定义用于读取配置文件的结构体。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub margin_mode: MarginMode,
    pub position_mode: PositionDirectionMode,
    pub position_margin_mode: PositionMarginMode,
    pub commission_level: CommissionLevel,
    pub current_commission_rate: CommissionRates,
    pub leverage_book: HashMap<String, f64>,
    pub fees_book: HashMap<String, f64>,
}


// 读取配置文件
#[allow(dead_code)]
pub fn read_config_file(file_path: &str) -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let config_content = std::fs::read_to_string(file_path)?;
    let config: ConfigFile = toml::from_str(&config_content)?;
    Ok(config)
}

// 从配置文件初始化 AccountConfig
impl From<ConfigFile> for AccountConfig {
    fn from(config: ConfigFile) -> Self {
        let leverage_book = config
            .leverage_book
            .into_iter()
            .map(|(k, v)| {
                let parts: Vec<&str> = k.split('_').collect();
                if parts.len() != 3 {
                    panic!("Invalid instrument format: {}", k);
                }
                (
                    Instrument::from((
                        parts[0].to_string(),
                        parts[1].to_string(),
                        InstrumentKind::from(parts[2].to_string()),
                    )),
                    v,
                )
            })
            .collect();

        let fees_book = config
            .fees_book
            .into_iter()
            .map(|(k, v)| (InstrumentKind::from(k), v))
            .collect();

        AccountConfig {
            margin_mode: config.margin_mode,
            position_mode: config.position_mode,
            position_margin_mode: config.position_margin_mode,
            commission_level: config.commission_level,
            current_commission_rate: config.current_commission_rate,
            leverage_book,
            fees_book,
        }
    }
}