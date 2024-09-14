use crate::{error::ExchangeError, sandbox::account::account_config::AccountConfig};
use std::{fs, path::Path};

/// 读取配置文件，并返回`AccountConfig`结构体实例。
///
/// 如果配置文件不存在或无法解析，将返回相应的`ExecutionError`。
///
/// # 错误
/// - `ExecutionError::ConfigMissing`: 如果配置文件 `config.toml` 不存在。
/// - `ExecutionError::ConfigParseError`: 如果TOML解析失败。
/// - `ExecutionError::InternalError`: 如果读取文件时发生IO错误。
pub fn read_config_file() -> Result<AccountConfig, ExchangeError>
{
    // 配置文件的路径
    let config_path = Path::new("config.toml");

    // 检查配置文件是否存在
    if !config_path.exists() {
        return Err(ExchangeError::ConfigMissing);
    }

    // 读取配置文件内容
    let config_content = fs::read_to_string(config_path).map_err(ExchangeError::from)?;

    // 解析TOML文件并转换为`AccountConfig`结构体
    let config: AccountConfig = toml::from_str(&config_content).map_err(ExchangeError::from)?;

    // 返回解析后的配置
    Ok(config)
}

// 将`std::io::Error`转换为自定义的`ExecutionError`
impl From<std::io::Error> for ExchangeError
{
    fn from(err: std::io::Error) -> Self
    {
        ExchangeError::InternalError(format!("IO error: {}", err))
    }
}

// 将TOML解析错误转换为自定义的`ExecutionError`
impl From<toml::de::Error> for ExchangeError
{
    fn from(err: toml::de::Error) -> Self
    {
        ExchangeError::ConfigParseError(format!("TOML error: {}", err))
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        common::{account_positions::PositionDirectionMode, instrument::kind::InstrumentKind},
        sandbox::account::account_config::{CommissionLevel, CommissionRates, MarginMode},
    };
    use std::{fs, io::Write};
    use tempfile::tempdir;

    /// 测试成功读取配置文件的情况
    #[test]
    fn test_read_config_file_success()
    {
        // 创建一个临时的TOML配置，符合`AccountConfig`结构体的定义
        let toml_content = r#"
    margin_mode = "SimpleMode"
    global_position_direction_mode = "Net"
    global_position_margin_mode = "Cross"
    commission_level = "Lv2"
    funding_rate = 0.0001
    global_leverage_rate = 100.0
    execution_mode = "Backtest"
    max_price_deviation = 0.05
    lazy_account_positions = false
    liquidation_threshold = 0.9



    [current_commission_rate]
    maker_fees = 0.001
    taker_fees = 0.002


    [fees_book]
    "spot" = { maker_fees = 0.001, taker_fees = 0.002 }
    "perpetual" = { maker_fees = 0.0005, taker_fees = 0.001 }
    "#;

        // 将TOML内容写入临时文件
        let config_path = Path::new("test_config.toml");
        let mut file = fs::File::create(&config_path).expect("Failed to create test config file");
        file.write_all(toml_content.as_bytes()).expect("Failed to write to test config file");
        file.sync_all().expect("Failed to sync test config file");
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 执行`read_config_file`函数
        let result = read_config_file();

        // 清理临时文件
        if config_path.exists() {
            fs::remove_file(config_path).expect("Failed to remove test config file");
        }
        else {
            eprintln!("File not found: {:?}", config_path);
        }

        // 断言结果
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let config = result.unwrap();

        assert_eq!(config.margin_mode, MarginMode::SimpleMode);
        assert_eq!(config.global_position_direction_mode, PositionDirectionMode::Net);
        assert_eq!(config.commission_level, CommissionLevel::Lv2);
        assert_eq!(config.global_leverage_rate, 1.0);
        assert_eq!(config.lazy_account_positions, false);
        assert_eq!(config.fees_book.get(&InstrumentKind::Spot).cloned(), Some(CommissionRates { maker_fees: 0.001, taker_fees: 0.002 }));
        assert_eq!(config.fees_book.get(&InstrumentKind::Perpetual).cloned(), Some(CommissionRates { maker_fees: 0.0005, taker_fees: 0.001 }));
    }

    /// 测试配置文件缺失的情况
    #[test]
    fn test_read_config_file_missing()
    {
        // 创建临时目录
        let dir = tempdir().unwrap();

        // 暂时切换当前目录到临时目录
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        // 调用函数并检查结果
        let config_result = read_config_file();
        assert!(matches!(config_result, Err(ExchangeError::ConfigMissing)), "Expected ConfigMissing error, got {:?}", config_result);

        // 将当前目录切换回原来的目录
        std::env::set_current_dir(original_dir).unwrap();
    }
}
