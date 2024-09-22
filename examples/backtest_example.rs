/// # Backtest Example Documentation
///
/// This code demonstrates how to run a local `HourglassExchange` using simulated market data.
/// The exchange communicates with a client that issues commands to simulate market events and
/// handles trading operations.
///
/// ## Overview
///
/// The main components involved in this code:
/// - **HourglassExchange**: Simulates market events and manages trading data.
/// - **HourglassClient**: Communicates with the exchange, sending commands and receiving market data.
/// - **ClickHouseClient**: Provides access to historical market data stored in a ClickHouse database.
///
/// The code runs in a loop where the client triggers the exchange to simulate the next market event
/// and processes the received data.
///
/// ## Components:
///
/// 1. **Channels for Communication**
///    The code sets up three channels to facilitate communication between components:
///    - `event_hourglass_tx`, `event_hourglass_rx`: Used to send and receive events from the exchange.
///    - `request_tx`, `request_rx`: Used by the client to send requests to the exchange.
///    - `market_tx`, `market_rx`: Used to send market data from the exchange to the client.
///
/// 2. **Client Initialization**
///    The `HourglassClient` is initialized with `request_tx` and `market_rx`, allowing it to send commands to the
///    exchange and listen for market events. It will later interact with the exchange by sending commands like `let_it_roll`.
///
/// 3. **Account Setup**
///    The `HourglassAccount` is created and wrapped in an `Arc<Mutex>` to allow safe concurrent access.
///    - The account is initialized with configurations, positions, balances, and an order book.
///    - The `single_level_order_books` hashmap stores bid/ask data for instruments like `ETH/USDT`.
///
/// 4. **ClickHouseClient and Market Data Source**
///    The `ClickHouseClient` is used to fetch historical market data from a ClickHouse database.
///    The data source for the exchange is set to a backtest mode where it reads data from ClickHouse using a cursor.
///
/// 5. **HourglassExchange Initialization**
///    The exchange is initialized using a builder pattern:
///    - `event_hourglass_rx` receives client commands.
///    - `account` stores the current state of the trading account.
///    - `data_source` provides the market data from ClickHouse for backtesting.
///    - `market_event_tx` is used to send market events to the client.
///
/// 6. **Running the Exchange**
///    The exchange is run locally using `tokio::spawn(hourglass_exchange.start())`, which listens for events such as
///    market data requests or trading operations.
///
/// 7. **Client-Exchange Interaction Loop**
///    In the main loop:
///    - The client calls `let_it_roll()` to trigger the exchange to process the next market event.
///    - The client then listens for the next piece of market data using `listen_for_market_data()`.
///    - If market data is received, it is processed (in this case, printed out).
///
/// ## Usage
///
/// This code is designed to run within a Tokio runtime, and the exchange operates in an asynchronous manner.
/// To run the backtest:
///
/// 1. Make sure that you have ClickHouse running and that the required data is available in the specified table.
/// 2. Use this code as an entry point to simulate market data and test trading strategies.
///
/// ```sh
/// cargo run --example backtest_example
/// ```
///
/// ## Example Output
/// ```
/// Successfully connected to the ClickHouse server.
/// Constructed query SELECT exchange, symbol, side, price, timestamp, amount FROM binance_futures_trades.binance_futures_trades_union_2024_05_05 ORDER BY timestamp DESC
/// Sent LetItRoll command successfully
/// Received market data: MarketTrade { symbol: "ETH/USDT", side: "buy", price: 16305.0, amount: 0.5 }
/// ```
///
/// ## Notes
///
/// - The `let_it_roll()` function triggers the next market data to be processed.
/// - The client listens for market data and processes it as needed.
/// - The ClickHouse client is responsible for fetching historical data and providing it to the exchange.

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
    ClientExecution,
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
};
use tokio::{
    sync::{mpsc, Mutex, RwLock},
};
use uuid::Uuid;
use hourglass::common::account_positions::{PositionDirectionMode, PositionMarginMode};
use hourglass::hourglass::account::account_config::{AccountConfig, CommissionLevel, HourglassMode, MarginMode};

#[tokio::main]
async fn main()
{
    // create the channels
    let (account_event_tx, _account_event_rx) = mpsc::unbounded_channel();
    let (client_event_tx, client_event_rx) = mpsc::unbounded_channel();
    let (market_event_tx, market_event_rx) = mpsc::unbounded_channel();

    #[allow(unused)]
    let mut hourglass_client = HourglassClient { client_event_tx: client_event_tx.clone(),
                                                 market_event_rx
    };

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
                                                             account_event_tx,
                                                             account_margin: Arc::new(Default::default()) }));

    // Sample cursor building
    let clickhouse_client = ClickHouseClient::new();
    let exchange = "binance";
    let instrument = "futures";
    let date = "2024_08_22";
    let cursor = clickhouse_client.cursor_unioned_public_trades(exchange, instrument, date).await.unwrap();

    // Initialize and configure HourglassExchange
    let hourglass_exchange = HourglassExchange::builder().event_hourglass_rx(client_event_rx)
                                                         .account(account_arc.clone())
                                                         .data_source(DataSource::Backtest(cursor))
                                                         .market_event_tx(market_event_tx)
                                                         .initiate()
                                                         .expect("Failed to build HourglassExchange");

    // Running the exchange in local mode in tokio runtime
    tokio::spawn(hourglass_exchange.start());
    // hourglass_client.let_it_roll().await.unwrap();

    loop {
        // Call next entry of data and handle potential errors
        if let Err(e) = hourglass_client.let_it_roll().await {
            eprintln!("Error executing LetItRoll: {:?}", e);
            break;
        }

        // Listen for market data
        if let Some(market_data) = hourglass_client.listen_for_market_data().await {
            // Process the market data
            // Your logic for handling market_data & customised trading strategy goes here?
            println!("Processed market data: {:?}", market_data);
        }
    }
}
