mod trade_jason_reader;

use dashmap::DashMap;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use hourglass::{
    common::{
        account_positions::{exited_positions::AccountExitedPositions, AccountPositions},
        balance::Balance,
        event::AccountEvent,
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, OrderId},
            order_instructions::OrderInstruction,
            states::{cancelled::Cancelled, open::Open, request_cancel::RequestCancel, request_open::RequestOpen},
            Order, OrderRole,
        },
        token::Token,
        Side,
    },
    hourglass::{
        account::{
            account_latency::{AccountLatency, FluctuationMode},
            account_orders::AccountOrders,
            HourglassAccount,
        },
        hourglass_client::HourglassClientEvent,
        HourglassExchange,
    },
    test_utils::create_test_account_configuration,
    Exchange,
};
use hourglass::hourglass::clickhouse_api::datatype::single_level_order_book::SingleLevelOrderBook;

/// Initializes and runs a sample exchange with predefined settings and a test order.
pub async fn run_sample_exchange(event_account_tx: mpsc::UnboundedSender<AccountEvent>, event_hourglass_rx: mpsc::UnboundedReceiver<HourglassClientEvent>)
{
    // Creating initial balances
    let balances = DashMap::new();
    let token1 = Token::from("ETH");
    let token2 = Token::from("USDT");
    balances.insert(token1.clone(), Balance::new(10000.0, 5000.0));
    balances.insert(token2.clone(), Balance::new(20000.0, 18000.0));

    // Creating initial positions with the updated structure
    let positions = AccountPositions::init();
    let closed_positions = AccountExitedPositions::init();

    let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
    let account_orders = AccountOrders::new(0, vec![instrument.clone()], AccountLatency { fluctuation_mode: FluctuationMode::Sine,
                                                                                          maximum: 10,  // Example value
                                                                                          minimum: -10, // Example value
                                                                                          current_value: 0 }).await;

    // Create and insert a test order
    let test_order = Order { instruction: OrderInstruction::Limit,
                             exchange: Exchange::Hourglass,
                             instrument: instrument.clone(),
                             timestamp: 1234124124124123, // Assumes a function to get current timestamp
                             cid: Some(ClientOrderId("test_cid".into())),
                             side: Side::Buy,
                             state: Open { id: OrderId(1234124124124123),
                                           price: 16499.0,
                                           size: 1.0,
                                           filled_quantity: 0.0,
                                           order_role: OrderRole::Maker } };

    // Directly modify the orders within the RwLock
    {
        let mut orders_guard = account_orders.instrument_orders_map.entry(instrument).or_default();
        let orders_write = orders_guard.value_mut(); // Assuming it's a DashMap
        orders_write.bids.push(test_order);
    }

    // Wrap the AccountOrders in Arc<RwLock> as required by HourglassAccount struct
    let orders_arc = Arc::new(RwLock::new(account_orders));
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
                                                             account_open_book: orders_arc,
        single_level_order_book: Arc::new(Mutex::new(single_level_order_books)),
                                                             balances,
                                                             positions,
                                                             exited_positions: closed_positions,
                                                             account_event_tx: event_account_tx,
                                                             account_margin: Arc::new(Default::default()) }));

    // Initialize and configure HourglassExchange
    let hourglass_exchange = HourglassExchange::builder().event_hourglass_rx(event_hourglass_rx)
                                                         .account(account_arc)
                                                         .initiate()
                                                         .expect("Failed to build HourglassExchange");

    // Running the exchange in local mode
    hourglass_exchange.run_local().await;
    println!("[run_default_exchange] : Hourglass exchange run successfully on local mode.");
}


/// 定义沙箱交易所支持的Instrument
#[allow(dead_code)]
pub async fn initial_balances() -> HashMap<Token, Balance>
{
    // 初始化账户余额
    let mut balances = HashMap::new();
    let token1 = Token::from("ETH");
    let token2 = Token::from("USDT");
    balances.insert(token1.clone(), Balance::new(10000.0, 5000.0));
    balances.insert(token2.clone(), Balance::new(20000.0, 18000.0));
    balances
}

/// 创建限价订单请求
pub fn order_request_limit<I>(instrument: I, cid: ClientOrderId, side: Side, price: f64, quantity: f64) -> Order<RequestOpen>
    where I: Into<Instrument>
{
    Order { instruction: OrderInstruction::Limit,
            exchange: Exchange::Hourglass,
            instrument: instrument.into(),
            timestamp: 1233312345124, // 使用当前时间戳
            cid: Some(cid.clone()),
            side,
            state: RequestOpen { reduce_only: false, // 假设创建的订单不是 reduce_only
                                 price,
                                 size: quantity } }
}

/// 创建开放订单
pub fn open_order<I>(instrument: I, cid: ClientOrderId, id: OrderId, side: Side, price: f64, quantity: f64, filled: f64) -> Order<Open>
    where I: Into<Instrument>
{
    Order { instruction: OrderInstruction::Limit,
            exchange: Exchange::Hourglass,
            instrument: instrument.into(),
            timestamp: 1233312345124, // 使用当前时间戳
            cid: Some(cid.clone()),
            side,
            state: Open { id,
                          price,
                          size: quantity,
                          filled_quantity: filled,
                          order_role: OrderRole::Maker } }
}

/// 创建订单取消请求
#[allow(dead_code)]
pub fn order_cancel_request<I, Id>(instrument: I, cid: ClientOrderId, side: Side, id: Id) -> Order<RequestCancel>
    where I: Into<Instrument>,
          Id: Into<OrderId>
{
    Order { instruction: OrderInstruction::Cancel,
            exchange: Exchange::Hourglass,
            instrument: instrument.into(),
            timestamp: 1234124124124123u64 as i64, // 使用当前时间戳
            cid: Some(cid),
            side,
            state: RequestCancel::from(id) }
}

/// 创建取消的订单
#[allow(dead_code)]
pub fn order_limit_cancelled<I, Id>(instrument: I, cid: ClientOrderId, side: Side, id: Id) -> Order<Cancelled>
    where I: Into<Instrument>,
          Id: Into<OrderId>
{
    Order { instruction: OrderInstruction::Limit,
            exchange: Exchange::Hourglass,
            instrument: instrument.into(),
            timestamp: 1234124124124123u64 as i64, // 使用当前时间戳
            cid: Some(cid),
            side,
            state: Cancelled::from(id) }
}
