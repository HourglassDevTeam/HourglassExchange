use dashmap::DashMap;
use hourglass::common::account_positions::exited_positions::AccountExitedPositions;
use hourglass::common::account_positions::AccountPositions;
use hourglass::common::instrument::kind::InstrumentKind;
use hourglass::common::instrument::Instrument;
use hourglass::common::token::Token;
use hourglass::hourglass::account::account_latency::{AccountLatency, FluctuationMode};
use hourglass::hourglass::account::account_orders::AccountOrders;
use hourglass::hourglass::account::HourglassAccount;
use hourglass::hourglass::clickhouse_api::datatype::single_level_order_book::SingleLevelOrderBook;
use hourglass::hourglass::clickhouse_api::queries_operations::ClickHouseClient;
use hourglass::hourglass::{HourglassExchange, DataSource};
use hourglass::test_utils::create_test_account_configuration;
use std::collections::HashMap;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;
use hourglass::ClientExecution;
use hourglass::hourglass::hourglass_client::HourglassClient;

#[tokio::main]
async fn main()
{

    #[allow(unused)]
    // create the channels
    let (event_hourglass_tx, event_hourglass_rx) = mpsc::unbounded_channel();
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let (market_tx, market_rx) = mpsc::unbounded_channel();

    #[allow(unused)]
    let hourglass_client = HourglassClient { request_tx: request_tx.clone(), market_event_rx: market_rx };

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

    // Instantiate HourglassAccount and wrap in Arc<Mutex> for shared access
    let account_arc = Arc::new(Mutex::new(HourglassAccount { current_session: Uuid::new_v4(),
        machine_id: 0,
        client_trade_counter: 0.into(),
        exchange_timestamp: AtomicI64::new(1234567),
        config: create_test_account_configuration(),
        account_open_book: Arc::new(RwLock::new(AccountOrders::new(0, vec![], AccountLatency {
            fluctuation_mode: FluctuationMode::Sine,
            maximum: 100,
            minimum: 2,
            current_value: 0,
        }).await)),
        single_level_order_book: Arc::new(Mutex::new(single_level_order_books)),
        balances:DashMap::new(),
        positions,
        exited_positions: closed_positions,
        account_event_tx: event_hourglass_tx,
        account_margin: Arc::new(Default::default()) }));

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
    // 启动 LetItRoll
    tokio::spawn(async move {
        if let Err(e) = hourglass_client.let_it_roll().await {
            eprintln!("Error executing LetItRoll: {:?}", e);
        }
    });
}
