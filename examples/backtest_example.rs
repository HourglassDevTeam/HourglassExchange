use crate::OrderType::Cancel;
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
        account_positions::{exited_positions::AccountExitedPositions, AccountPositions, PositionDirectionMode, PositionMarginMode},
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, OrderId},
            order_instructions::OrderInstruction,
            states::{request_cancel::RequestCancel, request_open::RequestOpen},
            Order,
        }
        ,
        token::Token,
        Side,
    },
    hourglass::{
        account::{
            account_config::{AccountConfig, CommissionLevel, HourglassMode, MarginMode},
            account_latency::{AccountLatency, FluctuationMode},
            account_orders::AccountOrders,
            HourglassAccount,
        },
        clickhouse_api::{
            datatype::{clickhouse_trade_data::MarketTrade, single_level_order_book::SingleLevelOrderBook},
            queries_operations::ClickHouseClient,
        },
        hourglass_client_local_mode::HourglassClient,
        DataSource, HourglassExchange,
    },
    ClientExecution, Exchange,
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

#[tokio::main]
async fn main()
{
    // create the channels
    let (account_event_tx, _account_event_rx) = mpsc::unbounded_channel();
    let (client_event_tx, client_event_rx) = mpsc::unbounded_channel();
    let (market_event_tx, market_event_rx) = mpsc::unbounded_channel();

    #[allow(unused)]
    let mut hourglass_client = HourglassClient { client_event_tx: client_event_tx.clone(),
                                                 market_event_rx };

    // Creating initial positions with the updated structure
    let positions = AccountPositions::init();
    let closed_positions = AccountExitedPositions::init();

    let mut single_level_order_books = HashMap::new();

    // FIXME mechanism to be updated to update `single_level_order_books` in
    single_level_order_books.insert(Instrument { base: Token::new("ETH".to_string()),
                                                 quote: Token::new("USDT".to_string()),
                                                 kind: InstrumentKind::Perpetual },
                                    SingleLevelOrderBook { latest_bid: 16305.0,
                                                           latest_ask: 16499.0,
                                                           latest_price: 0.0 });

    let hourglass_account_config = AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
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

    // initialise the tokens possibly to be traded
    let mut instruments: Vec<Instrument> = vec![];

    // initialise 1000PEPEUSDT
    instruments.push(Instrument { base: Token::from("1000PEPE"),
                                  quote: Token::from("USDT"),
                                  kind: InstrumentKind::Perpetual });

    // initialise 1000FLOKIUSDT
    instruments.push(Instrument { base: Token::from("1000FLOKI"),
                                  quote: Token::from("USDT"),
                                  kind: InstrumentKind::Perpetual });

    // Instantiate HourglassAccount and wrap in Arc<Mutex> for shared access
    let account_arc = Arc::new(Mutex::new(HourglassAccount { current_session: Uuid::new_v4(),
                                                             machine_id: 0,
                                                             client_trade_counter: 0.into(),
                                                             exchange_timestamp: AtomicI64::new(0),
                                                             config: hourglass_account_config,
                                                             account_open_book: Arc::new(RwLock::new(AccountOrders::new(0, instruments, AccountLatency { fluctuation_mode: FluctuationMode::Sine,
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
    let date = "2024_05_05";
    let cursor = clickhouse_client.cursor_unioned_public_trades_for_test(exchange, instrument, date).await.unwrap();

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

    let mut tokens_to_be_deposited: Vec<(Token, f64)> = Vec::new();

    // Create the Token instance
    let usdt_token = Token::from("USDT");

    // Push the tuple (Token, f64) into the vector
    tokens_to_be_deposited.push((usdt_token, 100000.0));

    // deposit 70000 USDT
    let _ = hourglass_client.deposit_tokens(tokens_to_be_deposited).await;
    let balance = hourglass_client.fetch_balances().await.unwrap();
    println!("Balance updated after deposit: {:?}", balance);

    let mut order_counter: i64 = 0;

    loop {
        // Call next entry of data and handle potential errors
        if let Err(e) = hourglass_client.let_it_roll().await {
            eprintln!("Error executing LetItRoll: {:?}", e);
            break;
        }

        // Listen for market data
        if let Some(market_data) = hourglass_client.listen_for_market_data().await {
            // Process the market data NOTE to be implemented.
            order_counter += 1;

            order_parser(&hourglass_client, &market_data, order_counter, &mut account_event_rx).await;


            // Your logic for handling market_data & customised trading strategy goes here?
            println!("Processed market data: {:?}", market_data);
        }
        else {
            break
        }
    }
}

#[allow(warnings)]
#[derive(Clone)]
struct Ids
{
    cid: ClientOrderId,
    id: OrderId,
}


#[allow(unused)]
impl Ids
{
    fn new(cid: ClientOrderId, id: OrderId) -> Self
    {
        Self { cid, id }
    }
}

pub async fn order_parser(
    client: &HourglassClient,
    trade: &MarketTrade,
    order_counter: i64,
    account_rx: &mut mpsc::UnboundedReceiver<AccountEvent>
)
{
    match account_rx.recv().await {
        | Some(AccountEvent { kind: AccountEventKind::OrdersOpen(new_orders),
                   .. }) => {
            println!("{:?}", new_orders[0].);
        }
        | other => {}
    }

    match mock_up_strategy(trade) {
        | Some(operation) => {
            match operation {
                | OrderType::Open(monk_order) => {
                    let order = Order { instruction: monk_order.order_type,                                                 // 订单指令
                                        exchange: Exchange::Hourglass,                                                      // 交易所
                                        instrument: Instrument::from(("1000PEPE", "USDT", InstrumentKind::Perpetual)),  // 交易工具
                                        timestamp: 1649192400000000,                                                        // 生成的时候填客户端下单时间,NOTE 回测场景中之后会被加上一个随机延迟时间。
                                        cid: Some(ClientOrderId(format!("{} {}", "PEPEbuy{}".to_string(), order_counter))), // 客户端订单ID
                                        side: monk_order.side,                                                              // 买卖方向
                                        state: RequestOpen { reduce_only: false,
                                                             price: monk_order.price,
                                                             size: monk_order.size } };

                    let new_orders = client.open_orders(vec![order]).await;
                    println!("[test_3] : {:?}", new_orders);
                }
                | OrderType::Cancel => {
                    let order_cancel = Order { instruction: OrderInstruction::Cancel,
                                               exchange: Exchange::Hourglass,
                                               instrument: Instrument::from(("1000PEPE", "USDT", InstrumentKind::Perpetual)),
                                               timestamp: 1649192400000000, // 使用当前时间戳
                                               cid: Some(ClientOrderId(format!("{} {}", "PEPEbuy{}".to_string(), order_counter))),
                                               side: Side::Buy,
                                               state: RequestCancel::from(Some(ClientOrderId("PEPEbuy".to_string()))) }; // gotta be parsed from an OrderID rather than ClientOrderID

                    let cancelled = client.cancel_orders(vec![order_cancel]).await;

                    println!("[test_5] : {:?}", cancelled);
                }
            }
        }
        | None => {}
    }
}

pub fn mock_up_strategy(trade: &MarketTrade) -> Option<OrderType>
{
    // parse the trade price
    let trade_price = trade.price;
    // // parse the trade size
    // let trade_size = trade.amount;
    // // parse the trade side
    // let trade_side = Side::from(trade.side.to_string().parse().unwrap());
    // the strategy's handling logic goes here
    match trade_price {
        | px if px == 1000.0 => {
            let operation = OrderType::Open(MockOrder { order_type: OrderInstruction::Limit,
                                                        side: Side::Buy,
                                                        price: 999.0,
                                                        size: 10.0 });

            Some(operation)
        }
        | px if px == 1050.0 => {
            // let operation = OrderType::Open(MockOrder { order_type: OrderInstruction::Limit,
            //     side: Side::Buy,
            //     price: 999.0,
            //     size: 10.0 });

            Some(Cancel)
        }
        | _ => None,
    }
}

pub enum OrderType
{
    Open(MockOrder),
    Cancel,
}

pub struct MockOrder
{
    order_type: OrderInstruction,
    side: Side,
    price: f64,
    size: f64,
}
