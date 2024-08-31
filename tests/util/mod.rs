use std::{
    collections::HashMap,
    sync::{atomic::AtomicI64, Arc, Weak},
    time::Duration,
};
use clickhouse::query::RowCursor;
use tokio::sync::{mpsc, Mutex};
use unilink_execution::common::event::AccountEvent;
use unilink_execution::sandbox::sandbox_client::SandBoxClientEvent;
use unilink_execution::sandbox::{SandBoxExchange};
use unilink_execution::test_utils::{create_test_account, create_test_account_config};
use unilink_execution::test_utils::create_test_account_orders;
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
        account_states::AccountState,
        Account,
    },
    Exchange,
};
use uuid::Uuid;
use unilink_execution::common::order::states::cancelled::Cancelled;
use unilink_execution::common::order::states::request_cancel::RequestCancel;

pub async fn run_default_exchange(
    event_simulated_rx: mpsc::UnboundedReceiver<SandBoxClientEvent>,
) {
    // Build and run the Sandbox Exchange
    let sandbox_exchange = SandBoxExchange::initiator()
        .event_sandbox_rx(event_simulated_rx)
        // .market_event_tx(None)
        .account(create_test_account().await) // 直接使用 create_test_account().await
        .initiate() // Use `initiate` instead of `build` for `SandBoxExchange`
        .expect("failed to build SandBoxExchange");

    println!("Sandbox exchange built successfully");

    // Run the exchange locally or online
    sandbox_exchange.run_local().await;
    println!("Sandbox exchange is running");
}

/// 设置延迟为50ms
pub fn latency_50ms() -> Duration {
    Duration::from_millis(50)
}

/// 设置交易手续费为0.5%
pub fn fees_50_percent() -> f64 {
    0.5
}

/// 定义沙箱交易所支持的Instrument
pub fn instruments() -> Vec<Instrument> {
    vec![Instrument::from(("TEST_BASE", "TEST_QUOTE", InstrumentKind::Perpetual))]
}
/// 初始化沙箱交易所账户余额
pub async fn initial_balances() -> Arc<Mutex<AccountState>> {
    // 初始化账户余额
    let mut balances = HashMap::new();
    balances.insert(Token::from("TEST_BASE"), Balance::new(10.0, 10.0, 1.0));
    balances.insert(Token::from("TEST_QUOTE"), Balance::new(10_000.0, 10_000.0, 1.0));

    let positions = AccountPositions {
        margin_pos: Vec::new(),
        perpetual_pos: Vec::new(),
        futures_pos: Vec::new(),
        option_pos: Vec::new(),
    };

    let account_state = AccountState {
        balances,
        positions,
        account_ref: Weak::new(),
    };
    let account_state_arc = Arc::new(Mutex::new(account_state));

    // 模拟环境中的 `Account`
    let account = Arc::new(Mutex::new(Account {
        current_session: Uuid::new_v4(),
        machine_id: 0,
        exchange_timestamp: AtomicI64::new(1234567),
        account_event_tx: tokio::sync::mpsc::unbounded_channel().0,
        config: Arc::new(create_test_account_config()),
        states: account_state_arc.clone(),
        orders: Arc::new(tokio::sync::RwLock::new(AccountOrders::new(0, vec![], AccountLatency {
            fluctuation_mode: FluctuationMode::Sine,
            maximum: 0,
            minimum: 0,
            current_value: 0,
        }).await)),
    }));

    // 更新 `account_ref` 以指向 `Account`
    {
        let mut account_state_locked = account_state_arc.lock().await;
        account_state_locked.account_ref = Arc::downgrade(&account);
    }

    // 直接返回 `Arc<Mutex<AccountState>>`
    account_state_arc
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
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
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
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: Open {
            id,
            price,
            size: quantity,
            filled_quantity: filled,
            order_role: OrderRole::Taker,
        },
    }
}

/// 创建订单取消请求
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
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: RequestCancel::from(id),
    }
}

/// 创建取消的订单
pub fn order_cancelled<I, Id>(
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
        kind: OrderInstruction::Cancel,
        exchange: Exchange::SandBox,
        instrument: instrument.into(),
        timestamp: chrono::Utc::now().timestamp_millis(), // 使用当前时间戳
        cid,
        side,
        state: Cancelled::from(id),
    }
}