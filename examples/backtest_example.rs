use hourglass::hourglass_log::warn;
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
use hourglass::common::balance::Balance;
use hourglass::{common::{
    account_positions::{exited_positions::AccountExitedPositions, AccountPositions, PositionDirectionMode, PositionMarginMode},
    instrument::{kind::InstrumentKind, Instrument},
    order::{
        identification::{client_order_id::ClientOrderId, OrderId},
        order_instructions::OrderInstruction,
        states::{request_cancel::RequestCancel, request_open::RequestOpen},
        Order,
    },
    token::Token,
    token_list::TOKEN_LIST,
    Side,
}, hourglass::{
    account::{
        account_config::{AccountConfig, CommissionLevel, HourglassMode, MarginMode},
        account_latency::{AccountLatency, FluctuationMode},
        account_orders::AccountOrders,
        HourglassAccount,
    },
    clickhouse_api::{datatype::clickhouse_trade_data::MarketTrade, queries_operations::ClickHouseClient},
    hourglass_client_local_mode::HourglassClient,
    DataSource, HourglassExchange,
}, hourglass_log, ClientExecution, Exchange};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;
use std::fmt::Display;

use log::{Level, LevelFilter, Record};
use time::Duration;
use hourglass::hourglass_log::{
    appender::{file::Period, FileAppender},
    info, LoggerGuard, LogFormat,
};


fn init() -> LoggerGuard {
    // TideFormatter定义了如何构建消息。
    // 由于将消息格式化为字符串可能会减慢日志宏调用的速度，习惯的方式是将所需字段原样发送到日志线程，然后在日志线程中构建消息。
    // Send 表示类型可以安全地在线程之间传递所有权，而 Sync 表示类型可以安全地在线程之间共享访问而不会引发数据竞争。
    // 在这里，Box<dyn Send + Sync + std::fmt::Display> 表示存储的对象需要是可以跨线程传递和共享访问的，并且必须实现 std::fmt::Display trait，以便可以将其格式化为字符串。
    struct Formatter;

    struct Msg {
        level: Level,
        // thread: Option<String>,
        // file_path: Option<&'static str>, // 这意味着这个file_path字符串引用是与整个程序的生命周期相同的，也就是说，在整个程序运行期间都有效。
        // line: Option<u32>,
        args: String,
        // module_path: Option<&'static str>,
    }

    impl LogFormat for Formatter {
        fn msg(&self, record: &Record) -> Box<dyn Send + Sync + std::fmt::Display> {
            Box::new(Msg {
                level: record.level(),
                // thread: std::thread::current().name().map(|n| n.to_string()),
                // file_path: record.file_static(),
                // line: record.line(),
                args: format!("{}", record.args()),
                // module_path: record.module_path_static(),
            })
        }
    }

    impl Display for Msg {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&format!(
                "[{}][{}]",
                // self.thread.as_ref().map(|x| x.as_str()).unwrap_or(""),
                // self.module_path.unwrap_or(""),
                // self.file_path.unwrap_or(""),
                // self.line.unwrap_or(0),
                self.level,
                self.args
            ))
        }
    }

    // let time_format =
    //     time::format_description::parse_owned::<1>("[month]月[day]日 [hour]时[minute]分[second]秒.[subsecond digits:6]").unwrap();
    hourglass_log::Builder::new()
        // 使用自定义格式TideLogFormat
        .format(Formatter)
        // 使用自定义的时间格式
        // .time_format(time_format)
        // 全局最大日志级别
        .max_log_level(LevelFilter::Info)
        // 定义 root appender, 传递 None 会写入到 stderr
        .root(
            FileAppender::builder()
                .path("./backtest.log")
                .rotate(Period::Minute)
                .expire(Duration::hours(8))
                .build(),
        )
        .try_init()
        .expect("logger build or set failed")
}


#[tokio::main]
async fn main()
{
    // init logger
    let _logger = init(); // 这行代码中的变量 _guard 并不是一个必须的命名，而是遵循 Rust 的命名约定和设计模式。
    info!("Backtest begins!");


    let token_balances: DashMap<Token, Balance> = DashMap::new();

    for token_str in &TOKEN_LIST {
        let token = Token::new(token_str.to_string());
        let balance = Balance::new(0.0, 0.0);
        token_balances.insert(token, balance);
    }

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

    let single_level_order_books = HashMap::new();

    // // FIXME mechanism to be updated to update `single_level_order_books` in
    // single_level_order_books.insert(Instrument { base: Token::new("ETH".to_string()),
    //                                              quote: Token::new("USDT".to_string()),
    //                                              kind: InstrumentKind::Perpetual },
    //                                 SingleLevelOrderBook { latest_bid: 16305.0,
    //                                                        latest_ask: 16499.0,
    //                                                        latest_price: 0.0 });

    let hourglass_account_config = AccountConfig { margin_mode: MarginMode::SingleCurrencyMargin,
                                                   global_position_direction_mode: PositionDirectionMode::Net,
                                                   global_position_margin_mode: PositionMarginMode::Cross,
                                                   commission_level: CommissionLevel::Lv1,
                                                   funding_rate: 0.0,
                                                   global_leverage_rate: 1.0,
                                                   fees_book: HashMap::new(),
                                                   execution_mode: HourglassMode::Backtest,
                                                   max_price_deviation: 0.1,
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
                                                             balances:token_balances,
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
    // let balance = hourglass_client.fetch_balances().await.unwrap();
    // info!("Balance updated after deposit: {:?}", balance);

    let mut order_ids = Vec::new();

    loop {
        // Call next entry of data and handle potential errors
        if let Err(e) = hourglass_client.let_it_roll().await {
            warn!("Error executing LetItRoll: {:?}", e);
            break;
        }

        // Listen for market data
        if let Some(market_data) = hourglass_client.listen_for_market_data().await {
            // Process the market data NOTE to be implemented.

            order_parser(&hourglass_client, &market_data, &mut order_ids).await;

            // Your logic for handling market_data & customised trading strategy goes here?
            info!("Processed market data: {:?}", market_data);
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

pub async fn order_parser(client: &HourglassClient, trade: &MarketTrade,  order_ids: &mut Vec<OrderId>)
{
    match mock_up_strategy(trade) {
        | Some(operation) => {
            match operation {
                | OrderType::Open(monk_order) => {
                    let order = Order { instruction: monk_order.order_type,                                                 // 订单指令
                                        exchange: Exchange::Hourglass,                                                      // 交易所
                                        instrument: Instrument::from(("1000PEPE", "USDT", InstrumentKind::Perpetual)),      // 交易工具
                                        timestamp: 1649192400000000,                                                        // 生成的时候填客户端下单时间,NOTE 回测场景中之后会被加上一个随机延迟时间。
                                        cid: None, // 客户端订单ID
                                        side: monk_order.side,                                                              // 买卖方向
                                        state: RequestOpen { reduce_only: false,
                                                             price: monk_order.price,
                                                             size: monk_order.size } };

                    let new_orders = client.open_orders(vec![order]).await;
                    info!("The new orders are : {:?}", &new_orders);

                    for order in new_orders {
                        match order {
                            | Ok(order) => order_ids.push(order.state.id),
                            | Err(e) => {
                                info!("{:?}", e);
                                return
                            }
                        }
                    }
                }
                | OrderType::Cancel => {
                    let order_cancel = Order { instruction: OrderInstruction::Cancel,
                                               exchange: Exchange::Hourglass,
                                               instrument: Instrument::from(("1000PEPE", "USDT", InstrumentKind::Perpetual)),
                                               timestamp: 1649192400000000, // 使用当前时间戳
                                               cid: None,
                                               side: Side::Buy,
                                               state: RequestCancel::from(order_ids[0].clone()) }; // gotta be parsed from an OrderID rather than ClientOrderID

                    let cancelled = client.cancel_orders(vec![order_cancel]).await;

                    info!("The cancelled orders are  : {:?}", cancelled);
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
        | px if px == 0.0086733 => {
            let operation = OrderType::Open(MockOrder { order_type: OrderInstruction::Limit,
                                                        side: Side::Buy,
                                                        price: 0.0085,
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
