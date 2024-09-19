# ⏳ Hourglass 

A simulated crypto exchange inspired by [Barter.rs](https://github.com/barter-rs/barter-rs).[**中文用户请移步**](https://github.com/arthur19q3/Hourglass/blob/master/README_CN.md)



## ⚠️ Warning

**HourGlass is under active development.** Please use with caution in production environments. Currently, only perpetual contracts and net positions are supported.

## ✨ Features

- **ClickHouse Integration**: Efficient storage and querying of trade data using ClickHouse, ensuring high performance for both real-time and backtesting scenarios.
- **Margin Modes**: Supports both Cross and Isolated margin modes, allowing users to manage risk across multiple positions.
- **Position Management**: Supports both Long/Short and Net positions, giving users flexibility in how they handle their trades.
- **Crypto Futures and Options**: Trade futures and options on various cryptocurrencies, allowing for more complex trading strategies.
- **Backtesting**: Supports both local and remote server-based backtesting, enabling users to test strategies with historical data.
- **High Performance**: Optimized for high-frequency trading scenarios with low-latency operations.
- **Configuration Parsing**: Supports parsing account settings from the `config.toml` file, allowing for flexible and easy configuration management.
- **Feishu Reporting**: Includes features for reporting to Feishu (with use cases available, though not fully implemented yet).
- **Position Closure Repository**: Supports a repository for managing and storing position closure data.
- **Multiple Stablecoins**: Supports multiple stablecoins, providing users with a variety of stable currency options.
- **Trade Data Backtesting**: Allows backtesting using only trade data, with future support planned for order book data.
- **Configuration Parsing**: Hourglass can now parse account settings directly from the `config.toml` file, streamlining the setup process.
- **Feishu Reporting**: Introduces the capability to report activities and metrics to Feishu, enhancing communication and monitoring (currently with use cases and partially implemented).
- **Position Closure Repository**: Implements a repository system to manage and store data related to the closure of positions.
- **Multiple Stablecoins**: Expands support to include various stablecoins, offering users more choices for their trading activities.
- **Trade Data Backtesting**: Enables backtesting using only trade data, with plans to incorporate order book data support in the future for more comprehensive strategy testing.
