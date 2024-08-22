use crate::{error::ExecutionError, sandbox::account::account_config::AccountConfig};
use std::{fs, path::Path};

// 读取配置文件
pub fn read_config_file() -> Result<AccountConfig, ExecutionError>
{
    let config_path = Path::new("config.toml");
    if !config_path.exists() {
        return Err(ExecutionError::ConfigMissing("config.toml not found in the project root directory".to_string()));
    }

    let config_content = fs::read_to_string(config_path).map_err(ExecutionError::from)?;
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
        ExecutionError::ConfigParseError(format!("TOML error: {}", err))
    }
}


#[cfg(test)]
mod tests {
    use crate::common_infrastructure::instrument::Instrument;
use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;
    use crate::common_infrastructure::instrument::kind::InstrumentKind;
    use crate::common_infrastructure::position::{PositionDirectionMode, PositionMarginMode};
    use crate::sandbox::account::account_config::{CommissionLevel, CommissionRates, MarginMode};

    #[test]
    fn test_read_config_file_success() {
        // Create a temporary TOML configuration that matches the AccountConfig struct
        let toml_content = r#"
    margin_mode = "SimpleMode"
    position_mode = "NetMode"
    position_margin_mode = "Isolated"
    commission_level = "Lv2"

    [current_commission_rate]
    maker_fees = 0.001
    taker_fees = 0.002

    [leverage_book.btc_usd_perpetual]
    base = "btc"
    quote = "usd"
    kind = "Perpetual"
    leverage = 100.0

    [leverage_book.eth_usd_perpetual]
    base = "eth"
    quote = "usd"
    kind = "Perpetual"
    leverage = 50.0

    [fees_book]
    "spot" = { maker_fees = 0.001, taker_fees = 0.002 }
    "perpetual" = { maker_fees = 0.0005, taker_fees = 0.001 }
    "#;

        // Write the TOML content to a temporary file
        let config_path = Path::new("test_config.toml");
        let mut file = fs::File::create(&config_path).expect("Failed to create test config file");
        file.write_all(toml_content.as_bytes())
            .expect("Failed to write to test config file");

        // Run the read_config_file function
        let result = read_config_file();

        // Clean up the temporary file
        fs::remove_file(config_path).expect("Failed to remove test config file");

        // Assert the result
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let config = result.unwrap();

        // Create Instruments to compare with
        let btc_usd_perpetual = Instrument::new("btc", "usd", InstrumentKind::Perpetual);
        let eth_usd_perpetual = Instrument::new("eth", "usd", InstrumentKind::Perpetual);


        assert_eq!(config.margin_mode, MarginMode::SimpleMode);
        assert_eq!(config.position_mode, PositionDirectionMode::NetMode);
        assert_eq!(config.position_margin_mode, PositionMarginMode::Isolated);
        assert_eq!(config.commission_level, CommissionLevel::Lv2);
        assert_eq!(config.current_commission_rate.maker_fees, 0.001);
        assert_eq!(config.current_commission_rate.taker_fees, 0.002);
        assert_eq!(config.account_leverage_rate, Some(100.0));
        assert_eq!(config.account_leverage_rate, Some(50.0));
        assert_eq!(
            config.fees_book.get(&InstrumentKind::Spot).cloned(),
            Some(CommissionRates {
                maker_fees: 0.001,
                taker_fees: 0.002
            })
        );
        assert_eq!(
            config.fees_book.get(&InstrumentKind::Perpetual).cloned(),
            Some(CommissionRates {
                maker_fees: 0.0005,
                taker_fees: 0.001
            })
        );
    }

    #[test]
    fn test_read_config_file_missing() {
        // Create a temporary directory
        let dir = tempdir().unwrap();

        // Temporarily change the current directory to the temp dir
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        // Call the function and check the result
        let config_result = read_config_file();
        assert!(
            matches!(config_result, Err(ExecutionError::ConfigMissing(_))),
            "Expected ConfigMissing error, got {:?}",
            config_result
        );

        // Change the directory back
        std::env::set_current_dir(original_dir).unwrap();
    }}
