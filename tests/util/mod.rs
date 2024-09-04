mod trade_jason_reader;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc},
    time::Duration,
};
use dashmap::DashMap;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use unilink_execution::common::order::states::cancelled::Cancelled;
use unilink_execution::common::order::states::request_cancel::RequestCancel;
use unilink_execution::sandbox::sandbox_client::SandBoxClientEvent;
use unilink_execution::sandbox::SandBoxExchange;
use unilink_execution::test_utils::create_test_account_config;
use unilink_execution::{
    common::{
        balance::Balance
        ,
        instrument::{kind::InstrumentKind, Instrument},
        order::{
            identification::{client_order_id::ClientOrderId, OrderId},
            order_instructions::OrderInstruction,
            states::{open::Open, request_open::RequestOpen},
            Order, OrderRole,
        },
        position::AccountPositions,
        token::Token,
        Side,
    },
    sandbox::account::{
        account_latency::{AccountLatency, FluctuationMode},
        account_orders::AccountOrders,
        Account,
    },
    Exchange,
};
use unilink_execution::common::event::AccountEvent;
/// Initializes and runs a sample exchange with predefined settings and a test order.
pub async fn run_sample_exchange(
    event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    event_sandbox_rx: mpsc::UnboundedReceiver<SandBoxClientEvent>,
) {
    // Creating initial balances
    let balances = DashMap::new();
    let token1 = Token::from("ETH");
    let token2 = Token::from("USDT");
    balances.insert(token1.clone(), Balance::new(100.0, 50.0, 1.0));
    balances.insert(token2.clone(), Balance::new(200.0, 150.0, 1.0));

    // Creating initial positions
    let positions = AccountPositions {
        margin_pos: Vec::new(),
        perpetual_pos: Vec::new(),
        futures_pos: Vec::new(),
        option_pos: Vec::new(),
    };

    let instrument = Instrument::from(("ETH", "USDT", InstrumentKind::Perpetual));
    let account_orders = AccountOrders::new(0, vec![instrument.clone()], AccountLatency {
        fluctuation_mode: FluctuationMode::Sine,
        maximum: 10, // Example value
        minimum: -10, // Example value
        current_value: 0,
    }).await;

    // Create and insert a test order
    let test_order = Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.clone(),
        timestamp: 1234124124124123, // Assumes a function to get current timestamp
        cid: ClientOrderId(Some("test_cid".into())),
        side: Side::Buy,
        state: Open {
            id: OrderId(1234124124124123),
            price: 100.0,
            size: 1.0,
            filled_quantity: 0.0,
            order_role: OrderRole::Maker,
        },
    };

    // Directly modify the orders within the RwLock
    {
        let mut orders_guard = account_orders.instrument_orders_map.entry(instrument).or_default();
        let orders_write = orders_guard.value_mut(); // Assuming it's a DashMap
        orders_write.bids.push(test_order);
    }



    // Wrap the AccountOrders in Arc<RwLock> as required by Account struct
    let orders_arc = Arc::new(RwLock::new(account_orders));

    // Instantiate Account and wrap in Arc<Mutex> for shared access
    let account_arc = Arc::new(Mutex::new(Account {
        current_session: Uuid::new_v4(),
        machine_id: 0,
        exchange_timestamp: AtomicI64::new(1234567),
        config: Arc::new(create_test_account_config()),
        orders: orders_arc,
        balances,
        positions,
        account_event_tx: event_account_tx,
    }));

    // Initialize and configure SandBoxExchange
    let sandbox_exchange = SandBoxExchange::initiator()
        .event_sandbox_rx(event_sandbox_rx)
        .account(account_arc)
        .initiate()
        .expect("Failed to build SandBoxExchange");

    // Running the exchange in local mode
    sandbox_exchange.run_local().await;
    println!("[run_default_exchange] : Sandbox exchange run successfully on local mode.");
}

/// 设置延迟为50ms
#[allow(dead_code)]
pub fn latency_50ms() -> Duration {
    Duration::from_millis(50)
}

/// 设置交易手续费为0.5%
#[allow(dead_code)]
pub fn fees_50_percent() -> f64 {
    0.5
}


/// 定义沙箱交易所支持的Instrument
#[allow(dead_code)]
pub async fn initial_balances() ->HashMap<Token, Balance> {
    // 初始化账户余额
    let mut balances = HashMap::new();
    let token1 = Token::from("ETH");
    let token2 = Token::from("USDT");
    balances.insert(token1.clone(), Balance::new(100.0, 50.0, 1.0));
    balances.insert(token2.clone(), Balance::new(200.0, 150.0, 1.0));
    balances
}

/// 创建限价订单请求
pub fn order_request_limit<I>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    price: f64,
    quantity: f64,
) -> Order<RequestOpen>
where
    I: Into<Instrument>,
{
    Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp:1233312345124, // 使用当前时间戳
        cid,
        side,
        state: RequestOpen {
            reduce_only: false, // 假设创建的订单不是 reduce_only
            price,
            size: quantity,
        },
    }
}

/// 创建开放订单
pub fn open_order<I>(
    instrument: I,
    cid: ClientOrderId,
    id: OrderId,
    side: Side,
    price: f64,
    quantity: f64,
    filled: f64,
) -> Order<Open>
where
    I: Into<Instrument>,
{
    Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: 1233312345124, // 使用当前时间戳
        cid,
        side,
        state: Open {
            id,
            price,
            size: quantity,
            filled_quantity: filled,
            order_role: OrderRole::Maker,
        },
    }
}

/// 创建订单取消请求
#[allow(dead_code)]
pub fn order_cancel_request<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<RequestCancel>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        kind: OrderInstruction::Cancel,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: 1234124124124123u64 as i64, // 使用当前时间戳
        cid,
        side,
        state: RequestCancel::from(id),
    }
}

/// 创建取消的订单
#[allow(dead_code)]
pub fn order_limit_cancelled<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<Cancelled>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        kind: OrderInstruction::Limit,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: 1234124124124123u64 as i64, // 使用当前时间戳
        cid,
        side,
        state: Cancelled::from(id),
    }
}