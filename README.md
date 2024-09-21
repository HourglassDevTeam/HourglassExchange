# ⏳ Hourglass

A simulated crypto exchange inspired by [Barter.rs](https://github.com/barter-rs/barter-rs).

[**中文版 README**](https://github.com/arthur19q3/Hourglass/blob/master/README_CN.md)

## ⚠️ Warning

**HourGlass is under active development.** Please use with caution in production environments. Currently, only perpetual contracts are supported.

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
- **Liquidation Mechanism**: Supports a liquidation mechanism with configurable liquidation thresholds, enabling automated risk management and position liquidation when certain conditions are met.



## 📜 Code Example

以下是 Hourglass 的代码示例：

```rust
use dashmap::DashMap;
use hourglass::{
    common::{
        account_positions::{exited_positions::AccountExitedPositions, AccountPositions},
        instrument::{kind::InstrumentKind, Instrument},
        token::Token,
    },
    hourglass::{
        account::{
            account_latency::{AccountLatency, FluctuationMode},
            account_orders::AccountOrders,
            HourglassAccount,
        },
        clickhouse_api::{datatype::single_level_order_book::SingleLevelOrderBook, queries_operations::ClickHouseClient},
        hourglass_client::HourglassClient,
        DataSource, HourglassExchange,
    },
    test_utils::create_test_account_configuration,
    ClientExecution,
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
    time::Duration,
};
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    time,
};
use uuid::Uuid;

#[tokio::main]
async fn main()
{
    let (event_hourglass_tx, event_hourglass_rx) = mpsc::unbounded_channel();
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let (market_tx, market_rx) = mpsc::unbounded_channel();

    let mut hourglass_client = HourglassClient { request_tx: request_tx.clone(), market_event_rx: market_rx };

    let positions = AccountPositions::init();
    let closed_positions = AccountExitedPositions::init();

    let mut single_level_order_books = HashMap::new();
    single_level_order_books.insert(Instrument { base: Token::new("ETH".to_string()), quote: Token::new("USDT".to_string()), kind: InstrumentKind::Perpetual }, SingleLevelOrderBook { latest_bid: 16305.0, latest_ask: 16499.0, latest_price: 0.0 });

    let account_arc = Arc::new(Mutex::new(HourglassAccount { current_session: Uuid::new_v4(), machine_id: 0, client_trade_counter: AtomicI64::new(0), ... }));
}
