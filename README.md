# ⏳ Hourglass

A simulated crypto exchange inspired by [Barter.rs](https://github.com/barter-rs/barter-rs).

[**中文版 README**](https://github.com/arthur19q3/Hourglass/blob/master/README_CN.md)

## Warnings

**⚠️️️ Hourglass supports Linux exclusively.** We apologise but windows systems are not supported as of yet.
**⚠️ HourGlass is under active development.** Please use with caution in production environments. Currently, only perpetual contracts are supported.

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
    #[allow(unused)]
    // create the channels
    let (event_hourglass_tx, event_hourglass_rx) = mpsc::unbounded_channel();
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let (market_tx, market_rx) = mpsc::unbounded_channel();

    #[allow(unused)]
    let mut hourglass_client = HourglassClient { request_tx: request_tx.clone(),
                                                 market_event_rx: market_rx };

    // Creating initial positions with the updated structure
    let positions = AccountPositions::init();
    let closed_positions = AccountExitedPositions::init();

    let mut single_level_order_books = HashMap::new();
    single_level_order_books.insert(Instrument { base: Token::new("ETH".to_string()),
                                                 quote: Token::new("USDT".to_string()),
                                                 kind: InstrumentKind::Perpetual },
                                    SingleLevelOrderBook { latest_bid: 16305.0,
                                                           latest_ask: 16499.0,
                                                           latest_price: 0.0 });

    let hourglass_account_config =  AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
        global_position_direction_mode: PositionDirectionMode::Net,
        global_position_margin_mode: PositionMarginMode::Cross,
        commission_level: CommissionLevel::Lv1,
        funding_rate: 0.0,
        global_leverage_rate: 1.0,
        fees_book: HashMap::new(),
        execution_mode: HourglassMode::Backtest,
        max_price_deviation: 0.05,
        lazy_account_positions: false,
        liquidation_threshold: 0.9 };


    // Instantiate HourglassAccount and wrap in Arc<Mutex> for shared access
    let account_arc = Arc::new(Mutex::new(HourglassAccount { current_session: Uuid::new_v4(),
                                                             machine_id: 0,
                                                             client_trade_counter: 0.into(),
                                                             exchange_timestamp: AtomicI64::new(1234567),
                                                             config: hourglass_account_config,
                                                             account_open_book: Arc::new(RwLock::new(AccountOrders::new(0, vec![], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                                                                                    maximum: 100,
                                                                                                                                                    minimum: 2,
                                                                                                                                                    current_value: 0 }).await)),
                                                             single_level_order_book: Arc::new(Mutex::new(single_level_order_books)),
                                                             balances: DashMap::new(),
                                                             positions,
                                                             exited_positions: closed_positions,
                                                             account_event_tx: event_hourglass_tx,
                                                             account_margin: Arc::new(Default::default()) }));

    // Sample cursor building
    let clickhouse_client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_05_05";
    let cursor = clickhouse_client.cursor_unioned_public_trades(exchange, instrument, date).await.unwrap();

    // Initialize and configure HourglassExchange
    let hourglass_exchange = HourglassExchange::builder().event_hourglass_rx(request_rx)
                                                         .account(account_arc.clone())
                                                         .data_source(DataSource::Backtest(cursor))
                                                         .market_event_tx(market_tx)
                                                         .initiate()
                                                         .expect("Failed to build HourglassExchange");

    // Running the exchange in local mode in tokio runtime
    tokio::spawn(hourglass_exchange.start());
    loop {
        // Call let_it_roll and handle potential errors
        if let Err(e) = hourglass_client.let_it_roll().await {
            eprintln!("Error executing LetItRoll: {:?}", e);
            break;
        }

        // Listen for market data
        if let Some(market_data) = hourglass_client.listen_for_market_data().await {
            // Process the market data
            // Your logic for handling market_data & customised trading strategy goes here
            println!("Processed market data: {:?}", market_data);
        }
    }
}
